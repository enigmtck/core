use reqwest::Client;
use reqwest::StatusCode;

use crate::activity_pub::{ApActor, ApObject};
use crate::db::create_remote_note;
use crate::db::Db;
use crate::models::leaders::get_leader_by_actor_ap_id_and_profile;
use crate::models::leaders::Leader;
use crate::models::profiles::Profile;
use crate::models::remote_actors::get_remote_actor_by_ap_id;
use crate::models::remote_actors::{create_or_update_remote_actor, NewRemoteActor, RemoteActor};
use crate::models::remote_notes::get_remote_note_by_ap_id;
use crate::models::remote_notes::NewRemoteNote;
use crate::signing::{sign, Method, SignParams};
use crate::webfinger::WebFinger;

use super::ApNote;

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

pub async fn get_note(conn: &Db, profile: Profile, id: String) -> Option<ApNote> {
    match get_remote_note_by_ap_id(conn, id.clone()).await {
        Some(remote_note) => Some(remote_note.into()),
        None => {
            let url = id.clone();
            let body = Option::None;
            let method = Method::Get;

            let signature = sign(SignParams {
                profile: profile.clone(),
                url,
                body,
                method,
            });

            let client = Client::new();
            match client
                .get(&id)
                .header("Signature", &signature.signature)
                .header("Date", signature.date)
                .header(
                    "Accept",
                    "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"",
                )
                .send()
                .await
            {
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

pub async fn get_actor(
    conn: &Db,
    id: String,
    profile: Option<Profile>,
) -> Option<(RemoteActor, Option<Leader>)> {
    match get_remote_actor_by_ap_id(conn, id.clone()).await {
        Some(remote_actor) => {
            if let Some(profile) = profile {
                Option::from((
                    remote_actor,
                    get_leader_by_actor_ap_id_and_profile(conn, id, profile.id).await,
                ))
            } else {
                Option::from((remote_actor, Option::None))
            }
        }
        None => {
            // let url = id.clone();
            // let body = Option::None;
            // let method = Method::Get;

            // let signature = sign(SignParams {
            //     profile,
            //     url,
            //     body,
            //     method,
            // });

            let client = Client::builder();
            let client = client.user_agent("Enigmatick/0.1").build().unwrap();
            let inter = client
                .get(&id)
                //    .header("Signature", &signature.signature)
                //    .header("Date", signature.date)
                // .header(
                //     "Accept",
                //     "application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\"",
                // )
                // I changed the above to the below to accommodate BirdsiteLive which sucks
                // it may make sense to revert and just deal with the fact that that service doesn't work right
                .header("Accept", "application/activity+json");

            //log::debug!("inter: {inter:#?}");

            match inter.send().await {
                Ok(resp) => match resp.status() {
                    StatusCode::ACCEPTED | StatusCode::OK => {
                        if let Ok(text) = resp.text().await {
                            if let Ok(actor) = serde_json::from_str::<ApActor>(&text) {
                                if let Some(remote) =
                                    create_or_update_remote_actor(conn, NewRemoteActor::from(actor))
                                        .await
                                {
                                    Option::from((remote, Option::None))
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
                    StatusCode::GONE => {
                        log::debug!("GONE: {:#?}", resp.status());
                        None
                    }
                    _ => {
                        log::debug!("STATUS: {:#?}", resp.status());
                        None
                    }
                },
                Err(e) => {
                    log::debug!("{e:#?}");
                    None
                }
            }
        }
    }
}

// pub async fn get_followers(conn: &Db, profile: Profile, id: String, page: Option<usize>) {
//     if let Some(actor) = get_actor(conn, id.clone(), Some(profile.clone())).await {
//         log::debug!("performing remote lookup for actor's followers");

//         let page = match page {
//             Some(x) => format!("{}?page={}", actor.0.followers, x),
//             None => actor.0.followers.to_string(),
//         };

//         let url = page.clone();
//         let body = Option::None;
//         let method = Method::Get;

//         let signature = sign(SignParams {
//             profile,
//             url,
//             body,
//             method,
//         });

//         let client = Client::new();
//         match client
//             .get(&page)
//             .header("Signature", &signature.signature)
//             .header("Date", signature.date)
//             .header(
//                 "Accept",
//                 "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"",
//             )
//             .send()
//             .await
//         {
//             Ok(resp) => {
//                 match resp.status() {
//                     StatusCode::ACCEPTED | StatusCode::OK => {
//                         let j: ApObject =
//                             serde_json::from_str(&resp.text().await.unwrap()).unwrap();
//                         log::debug!("followers\n{:#?}", j);
//                     }
//                     StatusCode::GONE => {
//                         log::debug!("GONE: {:#?}", resp.status());
//                         //Option::None;
//                     }
//                     _ => {
//                         log::debug!("STATUS: {:#?}", resp.status());
//                         //Option::None;
//                     }
//                 }
//             }
//             Err(e) => {
//                 log::debug!("{:#?}", e);
//                 //Option::None;
//             }
//         }
//     }
// }
