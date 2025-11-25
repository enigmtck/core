use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use deadpool_diesel::postgres::Pool;
use reqwest::StatusCode;
use scraper::Html;

use crate::db::runner::DbRunner;
use crate::events::EventChannels;
use crate::models::actors::{guaranteed_actor, Actor};
use crate::models::cache::Cache;
use crate::models::objects::{self, NewObject};
use crate::models::objects::{create_object, get_object_by_as_id, Object};
use crate::retriever::{get_actor, signed_get};
use crate::server::sanitize_json_fields;
use crate::{FetchReplies, ANCHOR_RE, HTTP_CLIENT};
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
// Size limit is now enforced at 10MB.
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

/// Fetches HTML from a URL and extracts metadata tags (Open Graph, Twitter Card, etc.)
async fn fetch_html_metadata(url: &str) -> Result<HashMap<String, String>> {
    const MAX_SIZE: usize = 10 * 1024 * 1024; // 10 MB limit

    // Fetch the HTML content using shared HTTP client
    let response = HTTP_CLIENT
        .get(url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    // Check content length to avoid downloading huge files
    if let Some(content_length) = response.content_length() {
        if content_length > MAX_SIZE as u64 {
            return Err(anyhow!("Content too large: {} bytes", content_length));
        }
    }

    // Download with size limit
    let bytes = response.bytes().await?;
    if bytes.len() > MAX_SIZE {
        return Err(anyhow!("Content too large: {} bytes", bytes.len()));
    }

    // Parse HTML
    let html = String::from_utf8_lossy(&bytes);
    let document = Html::parse_document(&html);

    // Extract meta tags
    let mut meta_map = HashMap::new();

    // Use global meta selector to avoid recreating parser
    use crate::META_SELECTOR;

    for element in document.select(&*META_SELECTOR) {
        // Handle property-based meta tags (Open Graph, etc.)
        if let Some(property) = element.value().attr("property") {
            if let Some(content) = element.value().attr("content") {
                meta_map.insert(property.to_string(), content.to_string());
            }
        }

        // Handle name-based meta tags (Twitter, description, etc.)
        if let Some(name) = element.value().attr("name") {
            if let Some(content) = element.value().attr("content") {
                meta_map.insert(name.to_string(), content.to_string());
            }
        }
    }

    Ok(meta_map)
}

async fn metadata_object(object: &Object) -> Vec<Metadata> {
    if let Some(as_content) = object.as_content.clone() {
        let links = get_links(as_content);
        let mut metadata_vec = Vec::new();

        for link in links {
            match fetch_html_metadata(&link).await {
                Ok(meta_map) => {
                    let metadata: Metadata = (link, meta_map).into();
                    metadata_vec.push(metadata);
                }
                Err(e) => {
                    log::debug!("Failed to fetch metadata for {}: {}", link, e);
                }
            }
        }

        metadata_vec
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
        log::warn!(
            "Max object processing depth ({}) reached for {}, stopping recursion",
            MAX_OBJECT_DEPTH,
            object.as_id
        );
        return Ok(object);
    }

    let metadata = metadata_object(&object).await;

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
                let _ =
                    handle_object(&conn, object.clone(), &mut HashSet::<String>::new(), 0).await;
            }
            _ => (),
        }
    }

    Ok(())
}
