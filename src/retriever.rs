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
use crate::models::leaders::{get_leader_by_actor_ap_id_and_profile, Leader};
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

pub async fn get_object(
    conn_opt: Option<&Db>,
    profile: Option<Actor>,
    id: String,
) -> Result<ApObject> {
    if let Ok(object_model) = get_object_by_as_id(conn_opt, id.clone()).await {
        let ap_object = ApObject::try_from(object_model)?
            .cache(conn_opt.expect("DB connection required for cache operation"))
            .await
            .clone()
            .load_ephemeral(
                conn_opt.expect("DB connection required for load_ephemeral"),
                None,
            )
            .await
            .clone();
        Ok(ap_object)
    } else {
        let db_conn_for_guaranteed = conn_opt.expect("DB connection required for guaranteed_actor");
        let resp = signed_get(
            guaranteed_actor(db_conn_for_guaranteed, profile).await,
            id,
            false,
        )
        .await?;

        if resp.status().is_success() {
            let text = resp.text().await?;
            let fetched_ap_object: ApObject = serde_json::from_str(&text)?;

            let db_conn_for_cache_and_create =
                conn_opt.expect("DB connection required for cache/create");
            let created_object_model = create_or_update_object(
                db_conn_for_cache_and_create,
                NewObject::try_from(
                    fetched_ap_object
                        .cache(db_conn_for_cache_and_create)
                        .await
                        .clone(),
                )?,
            )
            .await?;

            let mut final_ap_object = ApObject::try_from(created_object_model)?;

            Ok(final_ap_object
                .load_ephemeral(
                    conn_opt.expect("DB connection required for load_ephemeral"),
                    None,
                )
                .await
                .clone())
        } else {
            Err(anyhow!("unable to get_object"))
        }
    }
}

pub async fn get_local_or_cached_actor(
    conn_opt: Option<&Db>,
    id: String,
    requester: Option<Actor>,
) -> Result<Option<ApActor>> {
    match get_actor_by_as_id(conn_opt, id.clone()).await {
        Ok(actor_model) => {
            if actor_model.is_stale() {
                return Ok(None);
            }
            let db_conn_for_ret = conn_opt.expect("DB connection required for actor_ret");
            Ok(Some(
                actor_ret(db_conn_for_ret, requester, actor_model).await,
            ))
        }
        Err(_e) => Ok(None),
    }
}

pub async fn process_remote_actor_retrieval(
    conn_opt: Option<&Db>,
    profile: Option<Actor>,
    id: String,
) -> Result<ApActor> {
    let db_conn_for_guaranteed = conn_opt.expect("DB connection required for guaranteed_actor");
    let response = signed_get(
        guaranteed_actor(db_conn_for_guaranteed, profile.clone()).await,
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
    let mut new_actor_data = NewActor::try_from(
        actor_from_remote
            .cache(conn_opt.expect("DB conn for cache"))
            .await
            .clone(),
    )
    .map_err(anyhow::Error::msg)?;
    new_actor_data.ek_webfinger = webfinger;

    let actor_model = create_or_update_actor(conn_opt, new_actor_data)
        .await
        .context("Failed to create or update Actor")?;

    let db_conn_for_ret = conn_opt.expect("DB connection required for actor_ret");
    Ok(actor_ret(db_conn_for_ret, profile, actor_model).await)
}

async fn actor_ret(db_conn: &Db, requester: Option<Actor>, target: Actor) -> ApActor {
    if let Some(requester_actor) = requester.clone() {
        ApActor::from_actor_and_leader((
            target.clone().into(),
            get_leader_by_actor_ap_id_and_profile(
                db_conn,
                target.as_id.clone(),
                requester_actor.id,
            )
            .await,
        ))
    } else {
        target.into()
    }
}

pub async fn get_actor(
    conn_opt: Option<&Db>,
    id: String,
    requester: Option<Actor>,
    update: bool,
) -> Result<ApActor> {
    log::debug!("Retrieving: {id}");

    let actor_option = get_local_or_cached_actor(conn_opt, id.clone(), requester.clone()).await?;

    if let Some(actor) = actor_option {
        log::debug!("Locally retrieved Actor: {actor}");
        Ok(actor)
    } else if update {
        log::debug!("Retrieving remote Actor: {id}");
        process_remote_actor_retrieval(conn_opt, requester, id).await
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

    let mut request = client
        .get(url_str)
        .timeout(std::time::Duration::new(5, 0))
        .header("Accept", accept)
        .header("Signature", &signature.signature)
        .header("Date", signature.date);

    // Add media-specific headers only when accept_any is true (media downloads)
    if accept_any {
        request = request
            .header("Accept-Encoding", "identity") // Disable compression for video
            .header("Connection", "keep-alive")
            .timeout(std::time::Duration::from_secs(120)); // Longer timeout for large files

        // Don't add Range header for now - it's causing 206 responses that fail
    }

    request.send().await.map_err(anyhow::Error::msg)
}
