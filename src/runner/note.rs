use anyhow::{anyhow, Result};
use reqwest::StatusCode;
use webpage::{Webpage, WebpageOptions};

use crate::activity_pub::retriever::signed_get;
use crate::activity_pub::{ApHashtag, Metadata};
use crate::fairings::events::EventChannels;
use crate::models::actors::{guaranteed_actor, Actor};
use crate::models::cache::Cache;
use crate::models::objects;
use crate::models::objects::{create_or_update_object, get_object_by_as_id, Object};
use crate::{activity_pub::ApObject, db::Db, signing::Method};
use crate::{runner, ANCHOR_RE};
use serde_json::json;

use super::TaskError;

pub async fn fetch_remote_object(conn: &Db, id: String, profile: Actor) -> Option<Object> {
    let _url = id.clone();
    let _method = Method::Get;

    if let Ok(resp) = signed_get(profile, id, false).await {
        match resp.status() {
            StatusCode::ACCEPTED | StatusCode::OK => match resp.json().await {
                Ok(ApObject::Note(note)) => {
                    create_or_update_object(conn, note.cache(conn).await.clone().into())
                        .await
                        .ok()
                }
                Ok(ApObject::Question(question)) => {
                    create_or_update_object(conn, question.cache(conn).await.clone().into())
                        .await
                        .ok()
                }
                Err(e) => {
                    log::error!("FAILED TO DECODE REMOTE OBJECT\n{e:#?}");
                    None
                }
                _ => None,
            },
            StatusCode::GONE => {
                log::debug!("REMOTE OBJECT NO LONGER EXISTS AT SOURCE");
                None
            }
            _ => {
                log::error!("REMOTE OBJECT FETCH STATUS {:#?}", resp.status());
                log::error!("{:#?}", resp.text().await);
                None
            }
        }
    } else {
        None
    }
}

// TODO: This is problematic for links that point to large files; the filter tries
// to account for some of that, but that's not really a solution. Maybe a whitelist?
// That would suck. I wish the Webpage crate had a size limit (i.e., load pages with
// a maximum size of 10MB or whatever a reasonable amount would be).
fn get_links(text: String) -> Vec<String> {
    ANCHOR_RE
        .captures_iter(&text)
        .filter(|cap| {
            !cap[0].to_lowercase().contains("mention")
                && !cap[0].to_lowercase().contains("u-url")
                && !cap[0].to_lowercase().contains("hashtag")
                && !cap[0].to_lowercase().contains("download")
                && !cap[1].to_lowercase().contains(".pdf")
        })
        .map(|cap| cap[1].to_string())
        .collect()
}

fn metadata_object(object: &Object) -> Vec<Metadata> {
    if let Some(as_content) = object.as_content.clone() {
        get_links(as_content)
            .iter()
            .map(|link| {
                (
                    link.clone(),
                    Webpage::from_url(link, WebpageOptions::default()),
                )
            })
            .filter(|(_, metadata)| metadata.is_ok())
            .map(|(link, metadata)| (link, metadata.unwrap().html.meta).into())
            .collect()
    } else {
        vec![]
    }
}

pub async fn handle_object(
    conn: &Db,
    channels: Option<EventChannels>,
    mut object: Object,
    _announcer: Option<String>,
) -> anyhow::Result<Object> {
    let metadata = metadata_object(&object);

    if !metadata.is_empty() {
        object = objects::update_metadata(conn, object.id, serde_json::to_value(metadata).unwrap())
            .await?;
    }

    let hashtags: Vec<ApHashtag> = object.clone().into();

    if !hashtags.is_empty() {
        let hashtags = json!(hashtags
            .iter()
            .map(|x| x.name.clone().to_lowercase())
            .collect::<Vec<String>>());

        object = objects::update_hashtags(conn, object.id, hashtags)
            .await
            .unwrap_or(object);
    }

    let ap_object: ApObject = object.clone().try_into()?;
    let profile = guaranteed_actor(conn, None);

    if let ApObject::Note(note) = ap_object {
        let _ = runner::actor::get_actor(Some(conn), profile.await, note.attributed_to.to_string())
            .await;

        // if let Some(announcer) = announcer {
        //     note.ephemeral_announces = Some(vec![announcer]);
        // }

        note.cache(conn).await;

        if let Some(mut channels) = channels {
            channels.send(None, serde_json::to_string(&note.clone()).unwrap());
        }

        Ok(object)
    } else {
        Err(anyhow!("ApObject is not a Note"))
    }
}

// pub async fn handle_remote_encrypted_note_task(
//     _conn: Option<&Db>,
//     remote_note: RemoteNote,
// ) -> Result<()> {
//     log::debug!("adding to processing queue");

//     if let Some(ap_to) = remote_note.clone().ap_to {
//         cfg_if::cfg_if! {
//             if #[cfg(feature = "pg")] {
//                 let _to_vec: Vec<String> = serde_json::from_value(ap_to)?;
//             } else if #[cfg(feature = "sqlite")] {
//                 let _to_vec: Vec<String> = serde_json::from_str(&ap_to)?;
//             }
//         }

//         // need to refactor this because of the async in the closures
//         // to_vec
//         //     .iter()
//         //     .filter_map(|ap_id| get_profile_by_ap_id(conn, ap_id.to_string()).await)
//         //     .for_each(|profile| {
//         //         create_processing_item(None, (remote_note.clone(), profile.id).into()).await;
//         //     });
//     }

//     Ok(())
// }

pub async fn object_task(
    conn: Db,
    channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let ap_id = ap_ids.first().unwrap().clone();

    if let Ok(object) = get_object_by_as_id(Some(&conn), ap_id).await {
        cfg_if::cfg_if! {
            if #[cfg(feature = "pg")] {
                use crate::models::objects::ObjectType;

                if object.as_type == ObjectType::Note {
                    let _ = handle_object(&conn, channels, object.clone(), None).await;
                }
            }
            // else if #[cfg(feature = "sqlite")] {
            //     match remote_note.kind.as_str() {
            //         "note" => {
            //             let _ = handle_remote_note(conn, channels.clone(), remote_note.clone(), None).await;
            //         }
            //         "encrypted_note" => {
            //             let _ = handle_remote_encrypted_note_task(conn, remote_note).await;
            //         }
            //         _ => (),
            //     }
            // }
        }
    }

    Ok(())
}
