use anyhow::{anyhow, Result};
use deadpool_diesel::postgres::Pool;
use reqwest::StatusCode;
use webpage::{Webpage, WebpageOptions};

use crate::db::runner::DbRunner;
use crate::events::EventChannels;
use crate::models::actors::{guaranteed_actor, Actor};
use crate::models::cache::Cache;
use crate::models::objects;
use crate::models::objects::{create_or_update_object, get_object_by_as_id, Object};
use crate::retriever::{get_actor, signed_get};
use crate::server::sanitize_json_fields;
use crate::signing::Method;
use crate::ANCHOR_RE;
use jdt_activity_pub::{ApHashtag, ApObject, Metadata};
use serde_json::{json, Value};

use super::TaskError;

pub async fn fetch_remote_object<C: DbRunner>(
    conn: &C,
    id: String,
    profile: Actor,
) -> Result<Object> {
    let _url = id.clone();
    let _method = Method::Get;

    match signed_get(profile, id, false).await {
        Ok(resp) => match resp.status() {
            StatusCode::ACCEPTED | StatusCode::OK => {
                if let Ok(resp) = resp.json::<Value>().await {
                    let sanitized = sanitize_json_fields(resp.clone());
                    match serde_json::from_value::<ApObject>(sanitized) {
                        Ok(ApObject::Note(note)) => {
                            create_or_update_object(conn, note.cache(conn).await.clone().into())
                                .await
                        }
                        Ok(ApObject::Question(question)) => {
                            create_or_update_object(conn, question.cache(conn).await.clone().into())
                                .await
                        }
                        Ok(ApObject::Article(article)) => {
                            create_or_update_object(conn, article.cache(conn).await.clone().into())
                                .await
                        }
                        Err(e) => {
                            log::error!("Failed to decode remote Object: {e}");
                            Err(e.into())
                        }
                        _ => Err(anyhow!("Unimplemented ApObject")),
                    }
                } else {
                    Err(anyhow!("Failed to convert response to JSON"))
                }
            }
            StatusCode::GONE => {
                log::debug!("Remote Object no longer exists at source");
                Err(anyhow!("Object no longer exists"))
            }
            _ => {
                log::error!("Remote Object fetch status: {}", resp.status());
                match resp.text().await {
                    Ok(text) => log::error!("Remote Object fetch text: {text}"),
                    Err(e) => log::error!("Remote Object fetch error: {e}"),
                }
                Err(anyhow!("Unknown error"))
            }
        },
        Err(e) => Err(e),
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

pub async fn handle_object<C: DbRunner>(
    conn: &C,
    _channels: Option<EventChannels>,
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
    let profile = guaranteed_actor(conn, None).await;

    let _ = get_actor(
        conn,
        object
            .attributed_to()
            .first()
            .ok_or(anyhow!("Failed to identify attribution"))?
            .clone(),
        Some(profile),
        true,
    )
    .await;

    ap_object.cache(conn).await;

    Ok(object)
}

pub async fn object_task(
    pool: Pool,
    channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let ap_id = ap_ids.first().unwrap().clone();
    let conn = pool.get().await.map_err(|_| TaskError::TaskFailed)?;

    if let Ok(object) = get_object_by_as_id(&conn, ap_id).await {
        use crate::models::objects::ObjectType;

        if object.as_type == ObjectType::Note {
            let _ = handle_object(&conn, channels, object.clone(), None).await;
        }
    }

    Ok(())
}
