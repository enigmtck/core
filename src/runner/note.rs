use std::collections::HashSet;

use anyhow::{anyhow, Result};
use deadpool_diesel::postgres::Pool;
use reqwest::StatusCode;
use webpage::{Webpage, WebpageOptions};

use crate::db::runner::DbRunner;
use crate::events::EventChannels;
use crate::models::actors::{guaranteed_actor, Actor};
use crate::models::cache::Cache;
use crate::models::objects::{self, NewObject};
use crate::models::objects::{create_object, get_object_by_as_id, Object};
use crate::retriever::{get_actor, signed_get};
use crate::server::sanitize_json_fields;
use crate::{FetchReplies, ANCHOR_RE};
use jdt_activity_pub::{ApHashtag, ApObject, Metadata};
use serde_json::{json, Value};

use super::TaskError;

pub async fn fetch_remote_object<C: DbRunner>(
    conn: &C,
    id: String,
    profile: Actor,
) -> Result<Object> {
    let response = signed_get(profile, id, false).await?;

    match response.status() {
        StatusCode::ACCEPTED | StatusCode::OK => {
            let json = response.json::<Value>().await?;
            let sanitized = sanitize_json_fields(json);
            let ap_object = serde_json::from_value::<ApObject>(sanitized)?;

            let cached_object = match ap_object {
                ApObject::Note(note) => NewObject::from(note.cache(conn).await.clone()),
                ApObject::Question(question) => NewObject::from(question.cache(conn).await.clone()),
                ApObject::Article(article) => NewObject::from(article.cache(conn).await.clone()),
                _ => return Err(anyhow!("Unsupported ApObject type")),
            };

            create_object(conn, cached_object).await
        }
        StatusCode::GONE => {
            log::debug!("Remote Object no longer exists at source");
            Err(anyhow!("Object no longer exists"))
        }
        status => {
            log::error!("Remote Object fetch failed with status: {status}");
            if let Ok(text) = response.text().await {
                log::error!("Response body: {text}");
            }
            Err(anyhow!("Failed to fetch remote object: {status}"))
        }
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
    mut object: Object,
    visited: &mut HashSet<String>,
    depth: usize,
) -> anyhow::Result<Object> {
    const MAX_OBJECT_DEPTH: usize = 50;

    if depth >= MAX_OBJECT_DEPTH {
        log::warn!("Max object processing depth ({}) reached for {}, stopping recursion", MAX_OBJECT_DEPTH, object.as_id);
        return Ok(object);
    }

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
        Some(profile.clone()),
        true,
    )
    .await;

    ap_object.cache(conn).await;

    match ap_object {
        ApObject::Note(mut note) if note.replies.reference().is_some() => {
            let _ = Box::pin(note.fetch_replies(conn, visited, depth).await);
        }
        ApObject::Article(mut article) if article.replies.reference().is_some() => {
            let _ = Box::pin(article.fetch_replies(conn, visited, depth).await);
        }
        ApObject::Question(mut question) if question.replies.reference().is_some() => {
            let _ = Box::pin(question.fetch_replies(conn, visited, depth).await);
        }
        _ => (),
    }

    Ok(object)
}

pub async fn object_task(
    pool: Pool,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let ap_id = ap_ids.first().unwrap().clone();
    let conn = pool.get().await.map_err(|_| TaskError::TaskFailed)?;

    if let Ok(object) = get_object_by_as_id(&conn, ap_id).await {
        use crate::models::objects::ObjectType;

        match object.as_type {
            ObjectType::Note | ObjectType::Article | ObjectType::Question => {
                let _ = handle_object(&conn, object.clone(), &mut HashSet::<String>::new(), 0).await;
            }
            _ => (),
        }
    }

    Ok(())
}
