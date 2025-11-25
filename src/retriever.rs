use crate::db::runner::DbRunner;
use crate::models::activities::{get_activities_coalesced, TimelineFilters};
use anyhow::anyhow;
use anyhow::{Context, Result};
use jdt_activity_pub::{ActivityPub, ApActivity, ApCollection, ApObject, CollectionFetcher};
use reqwest::Response;
use url::Url;

use crate::models::actors::{
    create_or_update_actor, get_actor_by_as_id, guaranteed_actor, Actor, FromActorAndLeader,
    NewActor,
};
use crate::models::cache::Cache;
use crate::models::follows::get_follow;
use crate::models::objects::{create_object, get_object_by_as_id, NewObject};
use crate::signing::{sign, Method, SignParams};
use crate::webfinger::WebFinger;
use crate::{GetWebfinger, LoadEphemeral, WEBFINGER_RE};
use jdt_activity_pub::ApActor;

pub async fn activities<C: DbRunner>(
    conn: &C,
    limit: i32,
    min: Option<i64>,
    max: Option<i64>,
    profile: Option<Actor>,
    filters: TimelineFilters,
    base_url: Option<String>,
) -> ApObject {
    let server_url = format!("https://{}", *crate::SERVER_NAME);
    let base_url = base_url.unwrap_or(format!("{server_url}/inbox?page=true&limit={limit}"));

    let activities = get_activities_coalesced(
        conn,
        limit,
        min,
        max,
        profile,
        Some(filters),
        None,
        None,
        None,
    )
    .await
    .unwrap_or_default();

    let activities = activities
        .into_iter()
        .filter_map(|activity| match ApActivity::try_from(activity.clone()) {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{e}");
                None
            }
        })
        .map(ActivityPub::from)
        .collect();

    ApObject::Collection(ApCollection::from((activities, Some(base_url))))
}

pub async fn get_remote_collection_page<C: DbRunner>(
    conn: &C,
    profile: Option<Actor>,
    url: String,
) -> Result<ApCollection> {
    let response = signed_get(guaranteed_actor(conn, profile).await, url, false).await?;

    log::debug!("{response:?}");

    let raw = response.text().await?;
    let page: ApCollection = serde_json::from_str(&raw).map_err(anyhow::Error::msg)?;

    Ok(page.cache(conn).await.clone())
}

pub async fn get_remote_collection<C: DbRunner>(
    conn: &C,
    profile: Option<Actor>,
    url: String,
) -> Result<ApCollection> {
    let response = signed_get(guaranteed_actor(conn, profile).await, url, false).await?;

    let raw = response.text().await?;
    let page: ApCollection = serde_json::from_str(&raw).map_err(anyhow::Error::msg)?;

    Ok(page.cache(conn).await.clone())
}

pub async fn get_ap_id_from_webfinger(acct: String) -> Result<String> {
    let webfinger = get_remote_webfinger(acct).await?;

    webfinger
        .links
        .iter()
        .find_map(|link| {
            let kind_match = link.kind.as_deref()?;
            if kind_match == "application/activity+json"
                || kind_match
                    == "application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\""
            {
                link.href.clone()
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow!("Failed to find usable link"))
}

async fn get_remote_webfinger(handle: String) -> Result<WebFinger> {
    let captures = WEBFINGER_RE
        .captures_iter(&handle)
        .next()
        .ok_or("acct STRING NOT A WEBFINGER")
        .map_err(anyhow::Error::msg)?;

    // Ensure we have exactly 3 captures: the full match, username, and server
    if captures.len() != 3 {
        return Err(anyhow!("acct STRING NOT A WEBFINGER"));
    }

    let username = captures.get(1).map_or("", |m| m.as_str());
    let server = captures.get(2).map_or("", |m| m.as_str());

    let url = format!("https://{server}/.well-known/webfinger?resource=acct:{username}@{server}");

    let response = crate::HTTP_CLIENT
        .get(&url)
        .header("Accept", "application/jrd+json")
        .send()
        .await
        .map_err(anyhow::Error::msg)?;

    response.json().await.map_err(anyhow::Error::msg)
}

pub async fn get_object<C: DbRunner>(
    conn: &C,
    profile: Option<Actor>,
    id: String,
) -> Result<ApObject> {
    log::debug!("get_object called for id: {}", id);

    if let Ok(object_model) = get_object_by_as_id(conn, id.clone()).await {
        log::debug!("Object found in local database: {}", id);
        let ap_object = ApObject::try_from(object_model)?
            .cache(conn)
            .await
            .clone()
            .load_ephemeral(conn, profile)
            .await
            .clone();
        Ok(ap_object)
    } else {
        log::debug!("Object not in local database, fetching from remote: {}", id);

        let actor = guaranteed_actor(conn, profile.clone()).await;
        log::debug!("Using actor for signed request: {}", actor.as_id);

        let resp = signed_get(actor, id.clone(), false)
            .await
            .context(format!("Failed to perform signed GET for {}", id))?;

        let status = resp.status();
        log::debug!("Remote response status for {}: {}", id, status);

        if resp.status().is_success() {
            let text = resp
                .text()
                .await
                .context(format!("Failed to read response body for {}", id))?;

            log::debug!("Response body length for {}: {} bytes", id, text.len());

            let fetched_ap_object: ApObject = serde_json::from_str(&text).context(format!(
                "Failed to parse ApObject from response for {}: {}",
                id,
                if text.len() > 500 {
                    &text[..500]
                } else {
                    &text
                }
            ))?;

            // Log the actual variant name using Debug formatting
            let variant_name = format!("{:?}", fetched_ap_object)
                .split('(')
                .next()
                .unwrap_or("Unknown")
                .to_string();
            log::debug!("Successfully parsed ApObject variant: {}", variant_name);

            let cached = fetched_ap_object.cache(conn).await.clone();

            let created_object_model = match NewObject::try_from(cached) {
                Ok(new_obj) => create_object(conn, new_obj)
                    .await
                    .context(format!("Failed to create object in database for {}", id))?,
                Err(e) => {
                    // Save the problematic response to a trace file
                    let trace_path = format!(
                        "{}/trace_object_conversion_error_{}.json",
                        *crate::MEDIA_DIR,
                        chrono::Utc::now().timestamp()
                    );

                    if let Err(write_err) = std::fs::write(&trace_path, &text) {
                        log::warn!(
                            "Failed to write trace file to {}: {}",
                            trace_path,
                            write_err
                        );
                    } else {
                        log::info!("Saved problematic response to trace file: {}", trace_path);
                    }

                    return Err(anyhow::Error::msg(format!(
                        "Failed to convert {} (variant: {}) to NewObject: {}. Trace saved to: {}",
                        id, variant_name, e, trace_path
                    )));
                }
            };

            let mut final_ap_object = ApObject::try_from(created_object_model)?;

            log::debug!("Successfully stored and loaded object: {}", id);
            Ok(final_ap_object.load_ephemeral(conn, profile).await.clone())
        } else {
            let error_body = resp
                .text()
                .await
                .unwrap_or_else(|_| "(unable to read body)".to_string());
            let error_msg = format!(
                "Failed to fetch object from remote. URL: {}, Status: {}, Body: {}",
                id,
                status,
                if error_body.len() > 500 {
                    &error_body[..500]
                } else {
                    &error_body
                }
            );
            log::error!("{}", error_msg);
            Err(anyhow!(error_msg))
        }
    }
}

pub async fn get_local_or_cached_actor<C: DbRunner>(
    conn: &C,
    id: String,
    requester: Option<Actor>,
) -> Result<Option<ApActor>> {
    match get_actor_by_as_id(conn, id.clone()).await {
        Ok(actor_model) => {
            if actor_model.is_stale() {
                return Ok(None);
            }
            Ok(Some(actor_ret(conn, requester, actor_model).await))
        }
        Err(_e) => Ok(None),
    }
}

pub async fn process_remote_actor_retrieval<C: DbRunner>(
    conn: &C,
    profile: Option<Actor>,
    id: String,
) -> Result<ApActor> {
    let response = signed_get(
        guaranteed_actor(conn, profile.clone()).await,
        id.clone(),
        false,
    )
    .await?;

    if !response.status().is_success() {
        let message = response.text().await.unwrap_or_default();
        return Err(anyhow::Error::msg(format!(
            "Bad remote ApActor response: {message:#?}"
        )));
    }

    let text = response.text().await?;
    let actor_from_remote = serde_json::from_str::<ApActor>(&text)?;
    let webfinger = actor_from_remote.get_webfinger().await;
    let mut new_actor_data = NewActor::try_from(actor_from_remote.cache(conn).await.clone())
        .map_err(anyhow::Error::msg)?;
    new_actor_data.ek_webfinger = webfinger;

    let actor_model = create_or_update_actor(conn, new_actor_data)
        .await
        .context("Failed to create or update Actor")?;

    Ok(actor_ret(conn, profile, actor_model).await)
}

async fn actor_ret<C: DbRunner>(conn: &C, requester: Option<Actor>, target: Actor) -> ApActor {
    if let Some(requester_actor) = requester.clone() {
        let follow = get_follow(conn, requester_actor.as_id, target.as_id.clone())
            .await
            .ok();
        ApActor::from_actor_and_leader((target.clone().into(), follow))
    } else {
        target.into()
    }
}

pub async fn get_actor<C: DbRunner>(
    conn: &C,
    id: String,
    requester: Option<Actor>,
    update: bool,
) -> Result<ApActor> {
    log::debug!("Retrieving: {id}");

    let actor_option = get_local_or_cached_actor(conn, id.clone(), requester.clone()).await?;

    if let Some(actor) = actor_option {
        log::debug!("Locally retrieved Actor: {actor}");
        Ok(actor)
    } else if update {
        log::debug!("Retrieving remote Actor: {id}");
        process_remote_actor_retrieval(conn, requester, id).await
    } else {
        log::error!("Failed to retrieve Actor");
        Err(anyhow!("Failed to retrieve Actor"))
    }
}

pub async fn signed_get(profile: Actor, url: String, accept_any: bool) -> Result<Response> {
    let accept = if accept_any {
        "*/*"
    } else {
        "application/activity+json"
    };

    let url_str = &url.clone();

    let body = None;
    let method = Method::Get;
    let url = Url::parse(url_str)?;

    let signature = sign(SignParams {
        profile,
        url,
        body,
        method,
    })?;

    let mut request = crate::HTTP_CLIENT
        .get(url_str)
        .timeout(std::time::Duration::new(5, 0))
        .header("Accept", accept)
        .header("Signature", &signature.signature)
        .header("Date", signature.date);

    if accept_any {
        request = request
            .header("Accept-Encoding", "identity")
            .header("Connection", "keep-alive")
            .timeout(std::time::Duration::from_secs(120));
    }

    request.send().await.map_err(anyhow::Error::msg)
}

pub fn collection_fetcher() -> CollectionFetcher {
    Box::new(|url: &str| {
        let url = url.to_string();
        Box::pin(async move {
            crate::HTTP_CLIENT
                .get(&url)
                .header("Content-Type", "application/activity+json")
                .send()
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
                .json::<ApCollection>()
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        })
    })
}
