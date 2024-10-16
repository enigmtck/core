use anyhow::anyhow;
use anyhow::{Context, Result};
use reqwest::Client;
use reqwest::Response;
use reqwest::StatusCode;
use url::Url;

use crate::activity_pub::ApActor;
use crate::db::Db;
use crate::models::actors::{get_actor_by_as_id, guaranteed_actor, Actor};
use crate::models::cache::Cache;
use crate::models::leaders::get_leader_by_actor_ap_id_and_profile;
use crate::models::objects::{create_or_update_object, get_object_by_as_id, NewObject};
use crate::models::remote_actors::create_or_update_remote_actor;
use crate::models::remote_actors::NewRemoteActor;
use crate::runner::actor::update_actor_tags;
use crate::signing::{sign, Method, SignParams};
use crate::webfinger::WebFinger;
use crate::WEBFINGER_RE;

use super::{ApCollection, ApCollectionPage, ApObject};

pub async fn get_remote_collection_page(
    conn: &Db,
    profile: Option<Actor>,
    url: String,
) -> Result<ApCollectionPage> {
    let response = signed_get(guaranteed_actor(conn.into(), profile).await, url, false).await?;

    let raw = response.text().await?;
    log::debug!("REMOTE COLLECTION PAGE RESPONSE\n{raw}");
    let page: ApCollectionPage = serde_json::from_str(&raw).map_err(anyhow::Error::msg)?;

    //let page = response.json::<ApCollectionPage>().await?;
    Ok(page.cache(conn).await.clone())
}

pub async fn get_remote_collection(
    conn: &Db,
    profile: Option<Actor>,
    url: String,
) -> Result<ApCollection> {
    let response = signed_get(guaranteed_actor(conn.into(), profile).await, url, false).await?;

    let raw = response.text().await?;
    log::debug!("REMOTE COLLECTION RESPONSE\n{raw}");
    let page: ApCollection = serde_json::from_str(&raw).map_err(anyhow::Error::msg)?;

    //let page = response.json::<ApCollection>().await?;
    Ok(page.cache(conn).await.clone())
}

pub async fn get_ap_id_from_webfinger(acct: String) -> Option<String> {
    let webfinger = get_remote_webfinger(acct).await.ok()?;

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

    log::debug!("WEBFINGER URL: {url}");

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
        Some(object) => Some(ApObject::try_from(object).ok()?.cache(conn).await.clone()),
        None => match signed_get(guaranteed_actor(conn.into(), profile).await, id, false).await {
            Ok(resp) => match resp.status() {
                StatusCode::ACCEPTED | StatusCode::OK => {
                    let text = resp.text().await.ok()?;
                    let object = serde_json::from_str::<ApObject>(&text).ok()?;

                    create_or_update_object(
                        conn,
                        NewObject::try_from(object.cache(conn).await.clone()).ok()?,
                    )
                    .await
                    .ok()
                    .map(|x| ApObject::try_from(x).ok())?
                }
                _ => {
                    log::debug!("OBJECT FAILURE STATUS: {:#?}", resp.status());
                    None
                }
            },
            Err(e) => {
                log::error!("FAILED TO RETRIEVE OBJECT: {e:#?}");
                None
            }
        },
    }
}

pub async fn get_local_or_cached_actor(
    conn: &Db,
    id: String,
    requester: Option<Actor>,
    update: bool,
) -> Option<ApActor> {
    if let Some(actor) = get_actor_by_as_id(conn, id.clone()).await {
        if let Some(requester) = requester.clone() {
            Some(ApActor::from((
                actor,
                get_leader_by_actor_ap_id_and_profile(conn, id.clone(), requester.id).await,
            )))
        } else {
            Some(actor.into())
        }
    } else {
        None
    }
}

pub async fn process_remote_actor_retrieval(
    conn: &Db,
    profile: Option<Actor>,
    id: String,
) -> Result<ApActor> {
    let response = signed_get(guaranteed_actor(conn.into(), profile).await, id, false).await?;

    match response.status() {
        StatusCode::ACCEPTED | StatusCode::OK => {
            let text = response.text().await?;
            let actor = serde_json::from_str::<ApActor>(&text)?;
            let actor = NewRemoteActor::try_from(actor.cache(conn).await.clone())
                .map_err(anyhow::Error::msg)?;
            let remote_actor = create_or_update_remote_actor(conn.into(), actor)
                .await
                .context("failed to create or update remote actor")?;

            update_actor_tags(Some(conn), remote_actor.clone())
                .await
                .context("failed to create or update remote actor tags")?;

            Ok(remote_actor.into())
        }
        _ => Err(anyhow::Error::msg("bad response")),
    }
}

pub async fn get_actor(
    conn: &Db,
    id: String,
    requester: Option<Actor>,
    update: bool,
) -> Option<ApActor> {
    let actor = get_local_or_cached_actor(conn, id.clone(), requester.clone(), update).await;

    if let Some(actor) = actor {
        Some(actor.cache(conn).await.clone())
    } else if update {
        process_remote_actor_retrieval(conn, requester, id)
            .await
            .ok()
    } else {
        None
    }
}

pub async fn signed_get(profile: Actor, url: String, accept_any: bool) -> Result<Response> {
    log::debug!("RETRIEVING: {url}");
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

    log::debug!("SIGNING REQUEST FOR REMOTE RESOURCE");
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

    log::debug!("CLIENT REQUEST\n{client:#?}");

    client.send().await.map_err(anyhow::Error::msg)
}
