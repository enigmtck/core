use chrono::Duration;
use chrono::Utc;
use reqwest::Client;
use reqwest::Response;
use reqwest::StatusCode;

use crate::activity_pub::ApActor;
use crate::db::Db;
use crate::models::cache::cache_content;
use crate::models::cache::get_cache_item_by_url;
use crate::models::cache::NewCacheItem;
use crate::models::leaders::get_leader_by_actor_ap_id_and_profile;
use crate::models::profiles::get_profile_by_ap_id;
use crate::models::profiles::Profile;
use crate::models::remote_actors::get_remote_actor_by_ap_id;
use crate::models::remote_actors::{create_or_update_remote_actor, NewRemoteActor};
use crate::models::remote_notes::create_or_update_remote_note;
use crate::models::remote_notes::get_remote_note_by_ap_id;
use crate::models::remote_notes::NewRemoteNote;
use crate::signing::{sign, Method, SignParams};
use crate::webfinger::WebFinger;

use super::types::collection::ApCollectionPage;
use super::types::object::ApEmoji;
use super::ApAttachment;
use super::ApCollection;
use super::ApNote;
use super::ApTag;

pub async fn get_remote_collection_page(
    profile: Option<Profile>,
    url: String,
) -> Option<ApCollectionPage> {
    match maybe_signed_get(profile, url).await {
        Ok(response) => response.json::<ApCollectionPage>().await.ok(),
        _ => None,
    }
}

pub async fn get_remote_collection(profile: Option<Profile>, url: String) -> Option<ApCollection> {
    match maybe_signed_get(profile, url).await {
        Ok(response) => response.json::<ApCollection>().await.ok(),
        _ => None,
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
        None => match maybe_signed_get(profile, id).await {
            Ok(resp) => match resp.status() {
                StatusCode::ACCEPTED | StatusCode::OK => match resp.text().await {
                    Ok(n) => {
                        if let Ok(note) = serde_json::from_str::<ApNote>(&n) {
                            if let Some(attachment) = &note.attachment {
                                for x in attachment.iter() {
                                    if let ApAttachment::Document(document) = x {
                                        cache_content(conn, document.clone().into()).await;
                                    };
                                }
                            }

                            create_or_update_remote_note(conn, NewRemoteNote::from(note.clone()))
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

// This seems like I wrote it as a temporary fix to populate the webfinger stuff
// should probably remove it
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

// pub async fn get_actor(
//     conn: &Db,
//     id: String,
//     profile: Option<Profile>,
//     update: bool,
// ) -> Option<ApActor> {
//     let actor = {
//         // This checks to see if the request is for a local profile. Failing that,
//         // it checks to see if we've already captured the remote actor. It returns
//         // None otherwise.
//         if let Some(actor_profile) = get_profile_by_ap_id(conn, id.clone()).await {
//             if let Some(profile) = profile.clone() {
//                 Some(ApActor::from((
//                     actor_profile,
//                     get_leader_by_actor_ap_id_and_profile(conn, id.clone(), profile.id).await,
//                 )))
//             } else {
//                 Some(actor_profile.into())
//             }
//         } else if let Some(remote_actor) = get_remote_actor_by_ap_id(conn, id.clone()).await {
//             let now = Utc::now();
//             let updated = remote_actor.updated_at;

//             if update && now - updated > Duration::days(1) {
//                 None
//             } else if let Some(profile) = profile.clone() {
//                 Some(ApActor::from((
//                     remote_actor,
//                     get_leader_by_actor_ap_id_and_profile(conn, id.clone(), profile.id).await,
//                 )))
//             } else {
//                 Some(remote_actor.into())
//             }
//         } else {
//             None
//         }
//     };

//     if let Some(actor) = actor {
//         update_webfinger(conn, actor.id.clone().unwrap().to_string()).await;
//         Some(actor)
//     } else if update {
//         match maybe_signed_get(profile, id).await {
//             Ok(resp) => match resp.status() {
//                 StatusCode::ACCEPTED | StatusCode::OK => {
//                     if let Ok(text) = resp.text().await {
//                         if let Ok(actor) = serde_json::from_str::<ApActor>(&text) {
//                             if let Some(image) = actor.image.clone() {
//                                 if let Ok(cache_item) = NewCacheItem::try_from(image) {
//                                     if get_cache_item_by_url(conn, cache_item.url.clone())
//                                         .await
//                                         .is_none()
//                                     {
//                                         if let Some(cache_item) = cache_item.download().await {
//                                             create_cache_item(conn, cache_item).await;
//                                         }
//                                     }
//                                 };
//                             }

//                             if let Some(image) = actor.icon.clone() {
//                                 if let Ok(cache_item) = NewCacheItem::try_from(image) {
//                                     if get_cache_item_by_url(conn, cache_item.url.clone())
//                                         .await
//                                         .is_none()
//                                     {
//                                         if let Some(cache_item) = cache_item.download().await {
//                                             create_cache_item(conn, cache_item).await;
//                                         };
//                                     }
//                                 }
//                             }

//                             create_or_update_remote_actor(conn, NewRemoteActor::from(actor))
//                                 .await
//                                 .map(ApActor::from)
//                         } else {
//                             log::error!("UNABLE TO DECODE ACTOR\n{text}");
//                             None
//                         }
//                     } else {
//                         log::error!("UNABLE TO DECODE RESPONSE TO TEXT");
//                         None
//                     }
//                 }
//                 _ => {
//                     log::debug!("REMOTE ACTOR RETRIEVAL FAILED\n{:#?}", resp.text().await);
//                     None
//                 }
//             },
//             Err(e) => {
//                 log::debug!("FAILED TO RETRIEVE REMOTE ACTOR\n{e:#?}");
//                 None
//             }
//         }
//     } else {
//         None
//     }
// }

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

pub async fn handle_actor_images(conn: &Db, actor: &ApActor) {
    let mut emojis = if let Some(tags) = actor.tag.clone() {
        tags.into_iter()
            .filter_map(|x| {
                if let ApTag::Emoji(emoji) = x {
                    Some(emoji)
                } else {
                    None
                }
            })
            .map(|x| Some(x.icon))
            .collect()
    } else {
        vec![]
    };

    let mut images = vec![actor.image.clone(), actor.icon.clone()];
    images.append(&mut emojis);
    for image in images.into_iter().flatten() {
        cache_content(conn, image.clone().into()).await;
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
                        handle_actor_images(conn, &actor).await;
                        create_or_update_remote_actor(conn, actor.into())
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
        update_webfinger(conn, actor.id.clone().unwrap().to_string()).await;
        handle_actor_images(conn, &actor).await;
        Some(actor)
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
