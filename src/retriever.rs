use anyhow::anyhow;
use anyhow::{Context, Result};
use reqwest::Client;
use reqwest::Response;
use url::Url;

use crate::db::Db;
use crate::models::actors::{
    create_or_update_actor, get_actor_by_as_id, guaranteed_actor, Actor, FromActorAndLeader,
    NewActor,
};
use crate::models::cache::Cache;
use crate::models::leaders::get_leader_by_actor_ap_id_and_profile;
use crate::models::objects::{create_or_update_object, get_object_by_as_id, NewObject};
use crate::signing::{sign, Method, SignParams};
use crate::webfinger::WebFinger;
use crate::{GetWebfinger, LoadEphemeral, WEBFINGER_RE};
use jdt_activity_pub::ApActor;

use super::{ApCollection, ApObject};

pub async fn get_remote_collection_page(
    conn: &Db,
    profile: Option<Actor>,
    url: String,
) -> Result<ApCollection> {
    let response = signed_get(guaranteed_actor(conn, profile).await, url, false).await?;

    let raw = response.text().await?;
    let page: ApCollection = serde_json::from_str(&raw).map_err(anyhow::Error::msg)?;

    Ok(page.cache(conn).await.clone())
}

pub async fn get_remote_collection(
    conn: &Db,
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
        .filter_map(|x| {
            if x.kind == Some("application/activity+json".to_string())
                || x.kind
                    == Some(
                        "application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\""
                            .to_string(),
                    )
            {
                x.href.clone()
            } else {
                None
            }
        })
        .take(1)
        .next()
        .ok_or(anyhow!("Failed to find usable link"))
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

    let client = Client::builder()
        .user_agent("Enigmatick/0.1")
        .build()
        .map_err(anyhow::Error::msg)?;

    let response = client
        .get(&url)
        .header("Accept", "application/jrd+json")
        .send()
        .await
        .map_err(anyhow::Error::msg)?;

    response.json().await.map_err(anyhow::Error::msg)
}

pub async fn get_object(conn: &Db, profile: Option<Actor>, id: String) -> Option<ApObject> {
    match get_object_by_as_id(Some(conn), id.clone()).await.ok() {
        Some(object) => Some(
            ApObject::try_from(object)
                .ok()?
                .cache(conn)
                .await
                .clone()
                .load_ephemeral(conn, None)
                .await
                .clone(),
        ),
        None => {
            let resp = signed_get(guaranteed_actor(conn, profile).await, id, false)
                .await
                .ok()?;

            if resp.status().is_success() {
                let text = resp.text().await.ok()?;
                let object = serde_json::from_str::<ApObject>(&text).ok()?;

                let object = create_or_update_object(
                    conn,
                    NewObject::try_from(object.cache(conn).await.clone()).ok()?,
                )
                .await
                .ok()
                .map(|x| ApObject::try_from(x).ok())?;

                if let Some(mut object) = object {
                    Some(object.load_ephemeral(conn, None).await.clone())
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
}

pub async fn get_local_or_cached_actor(
    conn: &Db,
    id: String,
    requester: Option<Actor>,
) -> Option<ApActor> {
    let actor = get_actor_by_as_id(conn, id.clone()).await.ok()?;

    if actor.is_stale() {
        return None;
    }

    Some(actor_ret(conn, requester, actor).await)
}

pub async fn process_remote_actor_retrieval(
    conn: &Db,
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
        let message = response.text().await.ok();
        return Err(anyhow::Error::msg(format!(
            "Bad remote ApActor response: {message:#?}"
        )));
    }

    let text = response.text().await?;
    let actor = serde_json::from_str::<ApActor>(&text)?;
    let webfinger = actor.get_webfinger().await;
    let mut actor =
        NewActor::try_from(actor.cache(conn).await.clone()).map_err(anyhow::Error::msg)?;
    actor.ek_webfinger = webfinger;

    let actor = create_or_update_actor(Some(conn), actor)
        .await
        .context("Failed to create or update Actor")?;

    Ok(actor_ret(conn, profile, actor).await)
}

async fn actor_ret(conn: &Db, requester: Option<Actor>, target: Actor) -> ApActor {
    if let Some(requester) = requester.clone() {
        ApActor::from_actor_and_leader((
            target.clone().into(),
            get_leader_by_actor_ap_id_and_profile(conn, target.as_id.clone(), requester.id).await,
        ))
    } else {
        target.into()
    }
}

pub async fn get_actor(
    conn: &Db,
    id: String,
    requester: Option<Actor>,
    update: bool,
) -> Result<ApActor> {
    log::debug!("Retrieving: {id}");

    let actor = get_local_or_cached_actor(conn, id.clone(), requester.clone()).await;

    if let Some(actor) = actor {
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
    let client = Client::builder()
        .user_agent("Enigmatick/0.1")
        .build()
        .unwrap();

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

    let client = client
        .get(url_str)
        .timeout(std::time::Duration::new(5, 0))
        .header("Accept", accept)
        .header("Signature", &signature.signature)
        .header("Date", signature.date);

    client.send().await.map_err(anyhow::Error::msg)
}
