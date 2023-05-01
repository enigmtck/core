use chrono::Duration;
use chrono::Utc;
use reqwest::Client;
use reqwest::StatusCode;

use crate::activity_pub::ApActor;
use crate::db::create_remote_note;
use crate::db::Db;
use crate::models::leaders::get_leader_by_actor_ap_id_and_profile;
use crate::models::leaders::Leader;
use crate::models::profiles::get_profile_by_ap_id;
use crate::models::profiles::Profile;
use crate::models::remote_actors::get_remote_actor_by_ap_id;
use crate::models::remote_actors::{create_or_update_remote_actor, NewRemoteActor};
use crate::models::remote_notes::get_remote_note_by_ap_id;
use crate::models::remote_notes::NewRemoteNote;
use crate::signing::{sign, Method, SignParams};
use crate::webfinger::WebFinger;

use super::types::collection::ApCollectionPage;
use super::ApCollection;
use super::ApNote;

pub async fn get_remote_collection_page(page: String) -> Option<ApCollectionPage> {
    let client = Client::new();

    if let Ok(response) = client
        .get(page)
        .header("Accept", "application/activity+json")
        .send()
        .await
    {
        if let Ok(response) = response.json::<ApCollectionPage>().await {
            Some(response)
        } else {
            log::error!("FAILED TO PARSE RESPONSE AS ApCollectionPage");
            None
        }
    } else {
        log::error!("FAILED TO MAKE REMOTE REQUEST");
        None
    }
}

pub async fn get_remote_collection(url: String) -> Option<ApCollection> {
    let client = Client::new();

    if let Ok(response) = client
        .get(&url)
        .header("Accept", "application/activity+json")
        .send()
        .await
    {
        if let Ok(response) = response.json::<ApCollection>().await {
            Some(response)
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
        Some(remote_note) => Some(remote_note.into()),
        None => {
            let client = Client::new();
            let client = client.get(&id).header(
                "Accept",
                "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"",
            );

            let client = {
                if let Some(profile) = profile {
                    let url = id.clone();
                    let body = Option::None;
                    let method = Method::Get;

                    let signature = sign(SignParams {
                        profile,
                        url,
                        body,
                        method,
                    });

                    client
                        .header("Signature", &signature.signature)
                        .header("Date", signature.date)
                } else {
                    client
                }
            };

            match client.send().await {
                Ok(resp) => match resp.status() {
                    StatusCode::ACCEPTED | StatusCode::OK => match resp.text().await {
                        Ok(n) => {
                            let note = match serde_json::from_str::<ApNote>(&n) {
                                Ok(note) => Option::from(note),
                                Err(_) => {
                                    log::error!("FAILED TO DECODE REMOTE NOTE: {n:#?}");
                                    Option::None
                                }
                            };

                            if let Some(note) = note {
                                create_remote_note(conn, NewRemoteNote::from(note.clone())).await;
                                note.into()
                            } else {
                                log::error!("FAILED TO CREATE REMOTE NOTE: {n:#?}");
                                Option::None
                            }
                        }
                        Err(e) => {
                            log::error!("FAILED TO UNPACK REMOTE NOTE: {e:#?}");
                            Option::None
                        }
                    },
                    _ => {
                        log::debug!("REMOTE NOTE STATUS: {:#?}", resp.status());
                        Option::None
                    }
                },
                Err(e) => {
                    log::error!("FAILED TO RETRIEVE REMOTE NOTE: {e:#?}");
                    Option::None
                }
            }
        }
    }
}

pub async fn update_webfinger(conn: &Db, id: String) {
    if let Some(actor) = get_remote_actor_by_ap_id(conn, id).await {
        if actor.webfinger.is_none() {
            let actor = ApActor::from(actor);
            let remote_actor = NewRemoteActor::from(actor.clone());
            if create_or_update_remote_actor(conn, remote_actor)
                .await
                .is_some()
            {
                log::debug!("UPDATED {:#?}", actor.id);
            }
        }
    }
}

pub async fn get_actor(
    conn: &Db,
    id: String,
    profile: Option<Profile>,
    update: bool,
) -> Option<ApActor> {
    let actor = {
        // This checks to see if the request is for a local profile. Failing that,
        // it checks to see if we've already captured the remote actor. It returns
        // None otherwise.
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
            let updated = remote_actor.updated_at;

            if update && now - updated > Duration::days(7) {
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
    };

    if let Some(actor) = actor {
        update_webfinger(conn, actor.id.clone().unwrap().to_string()).await;
        Some(actor)
    } else if update {
        let client = Client::builder();
        let client = client.user_agent("Enigmatick/0.1").build().unwrap();

        let client = {
            if let Some(profile) = profile {
                let url = id.clone();
                let body = Option::None;
                let method = Method::Get;

                let signature = sign(SignParams {
                    profile,
                    url,
                    body,
                    method,
                });

                client
                    .get(&id)
                    .header("Signature", &signature.signature)
                    .header("Date", signature.date)
                    .header("Accept", "application/activity+json")
            } else {
                client
                    .get(&id)
                    .header("Accept", "application/activity+json")
            }
        };

        match client.send().await {
            Ok(resp) => match resp.status() {
                StatusCode::ACCEPTED | StatusCode::OK => {
                    if let Ok(text) = resp.text().await {
                        if let Ok(actor) = serde_json::from_str::<ApActor>(&text) {
                            if let Some(remote) =
                                create_or_update_remote_actor(conn, NewRemoteActor::from(actor))
                                    .await
                            {
                                Some(ApActor::from(remote))
                            } else {
                                None
                            }
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
                    log::debug!("REMOTE ACTOR RETRIEVAL STATUS: {:#?}", resp.status());
                    None
                }
            },
            Err(e) => {
                log::debug!("FAILED TO RETRIEVE REMOTE ACTOR\n{e:#?}");
                None
            }
        }
    } else {
        None
    }
}
