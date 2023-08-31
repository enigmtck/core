use chrono::Duration;
use chrono::Utc;
use reqwest::Client;
use reqwest::Response;
use reqwest::StatusCode;

use crate::activity_pub::ApActor;
use crate::db::Db;
use crate::models::cache::Cache;
use crate::models::leaders::get_leader_by_actor_ap_id_and_profile;
use crate::models::profiles::get_profile_by_ap_id;
use crate::models::profiles::Profile;
use crate::models::remote_actors::create_or_update_remote_actor;
use crate::models::remote_actors::get_remote_actor_by_ap_id;
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
    if let Ok(response) = maybe_signed_get(profile, url).await {
        if let Ok(page) = response.json::<ApCollectionPage>().await {
            Some(page.cache(conn).await.clone())
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn get_remote_collection(
    conn: &Db,
    profile: Option<Profile>,
    url: String,
) -> Option<ApCollection> {
    if let Ok(response) = maybe_signed_get(profile, url).await {
        if let Ok(page) = response.json::<ApCollection>().await {
            Some(page.cache(conn).await.clone())
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn get_ap_id_from_webfinger(acct: String) -> Option<String> {
    if let Some(webfinger) = get_remote_webfinger(acct).await {
        webfinger
            .links
            .iter()
            .filter_map(|x| {
                if x.kind == Some("application/activity+json".to_string())
                    || x.kind
                        == Some("application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\"".to_string())
                {
                    x.href.clone()
                } else {
                    None
                }
            })
            .take(1).next()
    } else {
        None
    }
}

pub async fn get_remote_webfinger(acct: String) -> Option<WebFinger> {
    let webfinger_re = regex::Regex::new(r#"@(.+?)@(.+)"#).unwrap();

    let mut username = Option::<String>::None;
    let mut server = Option::<String>::None;

    if webfinger_re.captures_len() == 3 {
        for cap in webfinger_re.captures_iter(&acct) {
            username = Option::from(cap[1].to_string());
            server = Option::from(cap[2].to_string());
        }
    }

    let username = username.unwrap_or_default();
    let server = server.unwrap_or_default();

    let url = format!("https://{server}/.well-known/webfinger/?resource=acct:{username}@{server}");

    let client = Client::new();

    if let Ok(response) = client.get(&url).send().await {
        let response: WebFinger = response.json().await.unwrap();
        Option::from(response)
    } else {
        Option::from(WebFinger::default())
    }
}

pub async fn get_note(conn: &Db, profile: Option<Profile>, id: String) -> Option<ApNote> {
    match get_remote_note_by_ap_id(conn, id.clone()).await {
        Some(remote_note) => Some(ApNote::from(remote_note).cache(conn).await.clone()),
        None => match maybe_signed_get(profile, id).await {
            Ok(resp) => match resp.status() {
                StatusCode::ACCEPTED | StatusCode::OK => match resp.text().await {
                    Ok(n) => {
                        if let Ok(note) = serde_json::from_str::<ApNote>(&n) {
                            create_or_update_remote_note(
                                conn,
                                NewRemoteNote::from(note.cache(conn).await.clone()),
                            )
                            .await
                            .map(ApNote::from)
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        log::error!("FAILED TO UNPACK REMOTE NOTE: {e:#?}");
                        None
                    }
                },
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
    } else if let Some(remote_actor) = get_remote_actor_by_ap_id(conn, id.clone()).await {
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
) -> Option<ApActor> {
    match maybe_signed_get(profile, id).await {
        Ok(resp) => match resp.status() {
            StatusCode::ACCEPTED | StatusCode::OK => {
                if let Ok(text) = resp.text().await {
                    if let Ok(actor) = serde_json::from_str::<ApActor>(&text) {
                        create_or_update_remote_actor(conn, actor.cache(conn).await.clone().into())
                            .await
                            .map(ApActor::from)
                    } else {
                        log::error!("UNABLE TO DECODE ACTOR\n{text}");
                        None
                    }
                } else {
                    log::error!("UNABLE TO DECODE RESPONSE TO TEXT");
                    None
                }
            }
            _ => {
                log::debug!("REMOTE ACTOR RETRIEVAL FAILED\n{:#?}", resp.text().await);
                None
            }
        },
        Err(e) => {
            log::debug!("FAILED TO RETRIEVE REMOTE ACTOR\n{e:#?}");
            None
        }
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
        process_remote_actor_retrieval(conn, profile, id).await
    } else {
        None
    }
}

pub async fn maybe_signed_get(
    profile: Option<Profile>,
    url: String,
) -> Result<Response, reqwest::Error> {
    let client = Client::builder();
    let client = client.user_agent("Enigmatick/0.1").build().unwrap();

    let client = {
        if let Some(profile) = profile {
            let body = Option::None;
            let method = Method::Get;

            log::debug!("SIGNING REQUEST FOR REMOTE RESOURCE");
            let signature = sign(SignParams {
                profile,
                url: url.clone(),
                body,
                method,
            });

            client
                .get(&url)
                .header("Signature", &signature.signature)
                .header("Date", signature.date)
                .header("Accept", "application/activity+json")
        } else {
            client
                .get(&url)
                .header("Accept", "application/activity+json")
        }
    };

    client.send().await
}
