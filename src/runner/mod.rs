use std::{collections::HashSet, error::Error, fmt::Debug};

use crate::db::runner::DbRunner;
use crate::events::EventChannels;
use crate::models::activities::get_activity_by_ap_id;
use crate::models::activities::TryFromExtendedActivity;
use crate::models::actors::tombstone_actor_by_as_id;
use crate::models::objects::tombstone_object_by_as_id;
use anyhow::{anyhow, Result};
use deadpool_diesel::postgres::Pool;
use futures_lite::Future;
use reqwest::Client;
use reqwest::Request;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use url::Url;

use crate::retriever::get_actor;
use crate::{
    models::{activities::add_log_by_as_id, actors::Actor, instances::get_instance_inboxes},
    signing::{Method, SignParams},
};
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{ApActivity, ApActor, ApAddress, ApObject};

use self::user::get_follower_inboxes;

pub mod announce;
pub mod cache;
pub mod note;
pub mod question;
pub mod search_index;
pub mod user;

//pub type DbConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

pub fn clean_text(text: String) -> String {
    let ammonia = ammonia::Builder::default();

    ammonia.clean(&text).to_string()
}

#[derive(Serialize, Clone)]
pub struct RequestInfo {
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: Option<String>,
}

fn request_builder_to_info(request: &Request) -> RequestInfo {
    let method = request.method().to_string();
    let url = request.url().to_string();
    let headers = request
        .headers()
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
        .collect();
    let body = request
        .body()
        .and_then(|body| body.as_bytes())
        .map(|bytes| String::from_utf8_lossy(bytes).to_string());

    RequestInfo {
        method,
        url,
        headers,
        body,
    }
}

#[derive(Clone, Serialize)]
pub struct LogMessage {
    pub code: Option<i32>,
    pub request: Option<RequestInfo>,
    pub response: Option<String>,
}

use tokio::task::JoinHandle;

pub async fn process_inbox(
    inbox: ApAddress,
    body: String,
    profile: Actor,
    client: Client,
) -> LogMessage {
    log::debug!("Sending to inbox: {inbox}");
    let url = match Url::parse(&inbox.to_string()) {
        Ok(url) => url,
        Err(e) => {
            return LogMessage {
                code: Some(-1),
                request: None,
                response: Some(e.to_string()),
            }
        }
    };

    let signature = match crate::signing::sign(SignParams {
        profile: profile.clone(),
        url,
        body: Some(body.clone()),
        method: Method::Post,
    }) {
        Ok(sig) => sig,
        Err(e) => {
            return LogMessage {
                code: Some(-1),
                request: None,
                response: Some(e.to_string()),
            }
        }
    };

    let request = client
        .post(inbox.to_string())
        .timeout(std::time::Duration::new(10, 0))
        .header("Date", signature.date)
        .header("Digest", signature.digest.unwrap())
        .header("Signature", &signature.signature)
        .header("Content-Type", "application/activity+json")
        .body(body)
        .build()
        .unwrap();

    let client_info = request_builder_to_info(&request);

    match client.execute(request).await {
        Ok(resp) => {
            let code = resp.status().as_u16();
            log::debug!("Send status: {code}");

            LogMessage {
                code: Some(code.into()),
                request: Some(client_info),
                response: resp.text().await.ok(),
            }
        }
        Err(e) => {
            log::error!("Failed to send to inbox: {inbox}");
            LogMessage {
                code: Some(-1),
                request: Some(client_info),
                response: Some(e.to_string()),
            }
        }
    }
}

async fn process_all_inboxes<C: DbRunner>(
    inboxes: Vec<ApAddress>,
    body: String,
    profile: Actor,
    conn: &C,
    as_id: String,
) -> Result<(), anyhow::Error> {
    let client = crate::HTTP_CLIENT.clone();

    let handles: Vec<JoinHandle<LogMessage>> = inboxes
        .into_iter()
        .map(|inbox| {
            let client = client.clone();
            let body = body.clone();
            let profile = profile.clone();

            tokio::spawn(process_inbox(inbox, body, profile, client))
        })
        .collect();

    let mut logs = Vec::new();
    for handle in handles {
        if let Ok(log) = handle.await {
            logs.push(log);
        }
    }

    if !logs.is_empty() {
        let logs = serde_json::to_value(&logs)?;
        add_log_by_as_id(conn, as_id, logs).await?;
    }

    Ok(())
}

pub async fn send_to_inboxes<C: DbRunner>(
    conn: &C,
    inboxes: Vec<ApAddress>,
    profile: Actor,
    message: ApActivity,
) -> Result<()> {
    let as_id = message.as_id().ok_or_else(|| {
        log::debug!("Message does not have an ID");
        anyhow!("Message does not have an ID")
    })?;

    // Debug: Check Update activity before serialization
    if let ApActivity::Update(ref update) = message {
        if let MaybeReference::Actual(ApObject::Note(ref note)) = update.object {
            log::debug!("ApNote.updated before serialization: {:?}", note.updated);
        }
    }

    let body = serde_json::to_string(&message).map_err(anyhow::Error::msg)?;
    log::debug!("Serialized body length: {}", body.len());

    // Debug: Check if 'updated' appears in serialized JSON
    if body.contains("\"updated\"") {
        log::debug!("✓ 'updated' field found in serialized JSON");
    } else {
        log::debug!("✗ 'updated' field NOT found in serialized JSON");
    }

    log::debug!("Processing inboxes: {inboxes:?}");
    process_all_inboxes(inboxes, body, profile, conn, as_id).await?;

    Ok(())
}

async fn handle_recipients<C: DbRunner>(
    conn: &C,
    inboxes: &mut HashSet<ApAddress>,
    sender: &Actor,
    address: &ApAddress,
) -> Result<()> {
    let actor = ApActor::from(sender.clone());

    if address.is_public() {
        inboxes.extend(get_instance_inboxes(conn).await?.into_iter());
    } else if let Some(followers) = actor.followers {
        if address.to_string() == followers {
            inboxes.extend(get_follower_inboxes(conn, sender.clone()).await);
        } else if let Ok(actor) = get_actor(
            conn,
            address.clone().to_string(),
            Some(sender.clone()),
            true,
        )
        .await
        {
            inboxes.insert(ApAddress::Address(actor.inbox));
        }
    }
    Ok(())
}

pub async fn get_inboxes<C: DbRunner>(
    conn: &C,
    activity: ApActivity,
    sender: Actor,
) -> Vec<ApAddress> {
    let mut inboxes = HashSet::<ApAddress>::new();

    let (to, cc) = match activity {
        ApActivity::Create(activity) => (activity.to.option(), activity.cc.option()),
        ApActivity::Delete(activity) => (activity.to.option(), activity.cc.option()),
        ApActivity::Announce(activity) => (activity.to.option(), activity.cc.option()),
        ApActivity::Update(activity) => (activity.to.option(), None),
        ApActivity::Like(activity) => (activity.to.option(), None),
        ApActivity::Follow(activity) => {
            if let MaybeReference::Reference(id) = activity.object {
                (Some(vec![ApAddress::Address(id)]), None)
            } else {
                (None, None)
            }
        }
        ApActivity::Undo(activity) => {
            if let MaybeReference::Actual(ref target_activity) = activity.object {
                match target_activity {
                    ApActivity::Follow(follow) => {
                        if let MaybeReference::Reference(target) = follow.object.clone() {
                            (Some(vec![ApAddress::Address(target)]), None)
                        } else {
                            (None, None)
                        }
                    }
                    ApActivity::Like(like) => (like.to.option(), None),
                    ApActivity::Announce(announce) => {
                        (announce.cc.option(), Some(vec![ApAddress::get_public()]))
                    }
                    _ => (None, None),
                }
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    };

    let consolidated = match (to, cc) {
        (Some(to), Some(cc)) => Some([to, cc].concat()),
        (Some(to), None) => Some(to),
        (None, Some(cc)) => Some(cc),
        (None, None) => None,
    };

    if let Some(consolidated) = consolidated {
        for address in consolidated.iter() {
            if let Err(e) = handle_recipients(conn, &mut inboxes, &sender, address).await {
                log::error!("Error handling recipient {address}: {e:?}");
                // Decide if you want to stop or continue. For now, we continue.
            }
        }
    }

    inboxes.into_iter().collect()
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum TaskError {
    TaskFailed,
    Prohibited,
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for TaskError {}

pub async fn run<Fut, F>(f: F, pool: Pool, channels: Option<EventChannels>, params: Vec<String>)
where
    F: Fn(Pool, Option<EventChannels>, Vec<String>) -> Fut,
    Fut: Future<Output = Result<(), TaskError>> + Send + 'static,
{
    tokio::spawn(f(pool, channels, params));
}

pub async fn send_activity_task(
    pool: Pool,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    use crate::models::actors::get_actor;
    let conn = pool.get().await.map_err(|_| TaskError::TaskFailed)?;

    for ap_id in ap_ids {
        let (activity, target_activity, target_object, target_actor) =
            get_activity_by_ap_id(&conn, ap_id.clone())
                .await
                .map_err(|e| {
                    log::error!("DB error retrieving activity {ap_id}: {e}");
                    TaskError::TaskFailed
                })?
                .ok_or_else(|| {
                    log::error!("Failed to retrieve Activity: {ap_id}");
                    TaskError::TaskFailed
                })?;

        let profile_id = activity.actor_id.ok_or(TaskError::TaskFailed)?;
        let sender = get_actor(&conn, profile_id).await.map_err(|_| {
            log::error!("Failed to retrieve Actor: {profile_id}");
            TaskError::TaskFailed
        })?;

        // Debug: Check target_object before reconstruction
        if let Some(ref obj) = target_object {
            log::debug!("target_object.as_updated from DB: {:?}", obj.as_updated);
        }

        let ap_activity = ApActivity::try_from_extended_activity((
            activity.clone(),
            target_activity,
            target_object,
            target_actor,
        ))
        .map_err(|e| {
            log::error!("Failed to build ApActivity: {e}");
            TaskError::TaskFailed
        })?
        .formalize();

        // Debug: Check reconstructed Update activity
        if let ApActivity::Update(ref update) = ap_activity {
            log::debug!("Reconstructed Update activity from DB");
            if let MaybeReference::Actual(ApObject::Note(ref note)) = update.object {
                log::debug!("ApNote.updated after reconstruction: {:?}", note.updated);
            }
        }

        if activity.kind.is_delete()
            && activity.target_actor_id.is_some()
            && activity.target_ap_id.is_some()
        {
            tombstone_actor_by_as_id(&conn, activity.target_ap_id.clone().unwrap())
                .await
                .map_err(|e| {
                    log::error!("Failure to Tombstone Actor: {e}");
                    TaskError::TaskFailed
                })?;
        }

        if activity.kind.is_delete()
            && activity.target_object_id.is_some()
            && activity.target_ap_id.is_some()
        {
            tombstone_object_by_as_id(&conn, activity.target_ap_id.unwrap())
                .await
                .map_err(|e| {
                    log::error!("Failure to Tombstone Object: {e}");
                    TaskError::TaskFailed
                })?;
        }

        let inboxes: Vec<ApAddress> = get_inboxes(&conn, ap_activity.clone(), sender.clone()).await;

        send_to_inboxes(&conn, inboxes, sender, ap_activity.clone())
            .await
            .map_err(|e| {
                log::error!("Failed to send Announce: {e}");
                TaskError::TaskFailed
            })?;
    }

    Ok(())
}
