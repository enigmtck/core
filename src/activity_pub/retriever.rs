use anyhow::{Context, Result};
use chrono::Duration;
use chrono::Utc;
use reqwest::Client;
use reqwest::Response;
use reqwest::StatusCode;
use url::Url;

use crate::activity_pub::ApActor;
use crate::db::Db;
use crate::models::cache::Cache;
use crate::models::leaders::get_leader_by_actor_ap_id_and_profile;
use crate::models::profiles::get_profile_by_ap_id;
use crate::models::profiles::guaranteed_profile;
use crate::models::profiles::Profile;
use crate::models::remote_actors::create_or_update_remote_actor;
use crate::models::remote_actors::get_remote_actor_by_ap_id;
use crate::models::remote_actors::NewRemoteActor;
use crate::models::remote_notes::create_or_update_remote_note;
use crate::models::remote_notes::get_remote_note_by_ap_id;
use crate::models::remote_notes::NewRemoteNote;
use crate::signing::{sign, Method, SignParams};
use crate::webfinger::WebFinger;

use super::ApCollection;
use super::ApCollectionPage;
use super::ApNote;

pub async fn get_remote_collection_page(
    conn: &Db,
    profile: Option<Profile>,
    url: String,
) -> Option<ApCollectionPage> {
    if let Ok(response) =
        signed_get(guaranteed_profile(conn.into(), profile).await, url, false).await
    {
        let page = response.json::<ApCollectionPage>().await.ok()?;
        Some(page.cache(conn).await.clone())
    } else {
        None
    }
}

pub async fn get_remote_collection(
    conn: &Db,
    profile: Option<Profile>,
    url: String,
) -> Option<ApCollection> {
    if let Ok(response) =
        signed_get(guaranteed_profile(conn.into(), profile).await, url, false).await
    {
        let page = response.json::<ApCollection>().await.ok()?;
        Some(page.cache(conn).await.clone())
    } else {
        None
    }
}

pub async fn get_ap_id_from_webfinger(acct: String) -> Option<String> {
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
}

pub async fn get_remote_webfinger(acct: String) -> Option<WebFinger> {
    let webfinger_re = regex::Regex::new(r#"@(.+?)@(.+)"#).unwrap();

    let mut username = Option::<String>::None;
    let mut server = Option::<String>::None;

    if webfinger_re.captures_len() == 3 {
        for cap in webfinger_re.captures_iter(&acct) {
            username = Some(cap[1].to_string());
            server = Some(cap[2].to_string());
        }
    }

    let username = username.unwrap_or_default();
    let server = server.unwrap_or_default();

    let url = format!("https://{server}/.well-known/webfinger/?resource=acct:{username}@{server}");

    let client = Client::new();

    if let Ok(response) = client.get(&url).send().await {
        let response: WebFinger = response.json().await.unwrap();
        Some(response)
    } else {
        Some(WebFinger::default())
    }
}

pub async fn get_note(conn: &Db, profile: Option<Profile>, id: String) -> Option<ApNote> {
    match get_remote_note_by_ap_id(conn, id.clone()).await {
        Some(remote_note) => Some(ApNote::from(remote_note).cache(conn).await.clone()),
        None => match signed_get(guaranteed_profile(conn.into(), profile).await, id, false).await {
            Ok(resp) => match resp.status() {
                StatusCode::ACCEPTED | StatusCode::OK => {
                    let text = resp.text().await.ok()?;
                    let note = serde_json::from_str::<ApNote>(&text).ok()?;

                    create_or_update_remote_note(
                        conn,
                        NewRemoteNote::from(note.cache(conn).await.clone()),
                    )
                    .await
                    .map(ApNote::from)
                }
                _ => {
                    log::debug!("REMOTE NOTE FAILURE STATUS: {:#?}", resp.status());
                    None
                }
            },
            Err(e) => {
                log::error!("FAILED TO RETRIEVE REMOTE NOTE: {e:#?}");
                None
            }
        },
    }
}

pub async fn get_local_or_cached_actor(
    conn: &Db,
    id: String,
    profile: Option<Profile>,
    update: bool,
) -> Option<ApActor> {
    if let Some(actor_profile) = get_profile_by_ap_id(conn, id.clone()).await {
        if let Some(profile) = profile.clone() {
            Some(ApActor::from((
                actor_profile,
                get_leader_by_actor_ap_id_and_profile(conn, id.clone(), profile.id).await,
            )))
        } else {
            Some(actor_profile.into())
        }
    } else if let Ok(remote_actor) = get_remote_actor_by_ap_id(conn.into(), id.clone()).await {
        let now = Utc::now();
        let updated = remote_actor.checked_at;

        if update && now - updated > Duration::days(1) {
            None
        } else if let Some(profile) = profile.clone() {
            Some(ApActor::from((
                remote_actor,
                get_leader_by_actor_ap_id_and_profile(conn, id.clone(), profile.id).await,
            )))
        } else {
            Some(remote_actor.into())
        }
    } else {
        None
    }
}

pub async fn process_remote_actor_retrieval(
    conn: &Db,
    profile: Option<Profile>,
    id: String,
) -> Result<ApActor> {
    let response = signed_get(guaranteed_profile(conn.into(), profile).await, id, false).await?;

    match response.status() {
        StatusCode::ACCEPTED | StatusCode::OK => {
            let text = response.text().await?;
            let actor = serde_json::from_str::<ApActor>(&text)?;
            let actor = NewRemoteActor::try_from(actor.cache(conn).await.clone())
                .map_err(anyhow::Error::msg)?;
            create_or_update_remote_actor(conn.into(), actor)
                .await
                .ok()
                .map(ApActor::from)
                .context("failed to create or update remote actor")
        }
        _ => Err(anyhow::Error::msg("bad response")),
    }
}

pub async fn get_actor(
    conn: &Db,
    id: String,
    profile: Option<Profile>,
    update: bool,
) -> Option<ApActor> {
    let actor = get_local_or_cached_actor(conn, id.clone(), profile.clone(), update).await;

    if let Some(actor) = actor {
        Some(actor.cache(conn).await.clone())
    } else if update {
        process_remote_actor_retrieval(conn, profile, id).await.ok()
    } else {
        None
    }
}

pub async fn signed_get(profile: Profile, url: String, accept_any: bool) -> Result<Response> {
    let client = Client::builder();
    let client = client.user_agent("Enigmatick/0.1").build().unwrap();

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

    client
        .get(url_str)
        .timeout(std::time::Duration::new(5, 0))
        .header("Accept", accept)
        .header("Signature", &signature.signature)
        .header("Date", signature.date)
        .send()
        .await
        .map_err(anyhow::Error::msg)
}
