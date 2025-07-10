use crate::db::runner::DbRunner;
use crate::models::actors::guaranteed_actor;
use crate::retriever::signed_get;
use crate::schema::cache;
use anyhow::{anyhow, Context, Result}; // Add Context
use bytes::Bytes as ReqwestBytes;
use chrono::{DateTime, Utc};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::Insertable;
use diesel::{sql_query, sql_types::Text, AsChangeset, Identifiable, Queryable, QueryableByName};
use jdt_activity_pub::ApArticle;
use jdt_activity_pub::MaybeMultiple;
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{
    ActivityPub, ApActivity, ApActor, ApAttachment, ApCollection, ApDocument, ApImage, ApNote,
    ApObject, ApQuestion, ApTag, Collectible,
};
use reqwest::header::CONTENT_TYPE;
use reqwest::StatusCode as ReqwestStatusCode;
use serde::{Deserialize, Serialize};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::time::{sleep, Duration as TokioDuration};

use super::actors::{get_actor_by_username, Actor};

#[derive(
    Identifiable, Queryable, QueryableByName, AsChangeset, Serialize, Clone, Default, Debug,
)]
#[diesel(table_name = cache)]
pub struct CacheItem {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub url: String,
    pub media_type: Option<String>,
    pub height: Option<i32>,
    pub width: Option<i32>,
    pub blurhash: Option<String>,
    pub path: Option<String>,
}

pub trait Cache {
    async fn cache<C: DbRunner + Send + Sync>(&self, conn: &C) -> &Self;
}

pub enum Cacheable {
    Document(ApDocument),
    Image(ApImage),
}

impl Cache for ApCollection {
    async fn cache<C: DbRunner + Send + Sync>(&self, conn: &C) -> &Self {
        let items = self.items().unwrap_or_default();
        for item in items {
            if let ActivityPub::Activity(ApActivity::Create(create)) = item {
                if let MaybeReference::Actual(ApObject::Note(note)) = create.object {
                    note.cache(conn).await;
                }
            }
        }

        self
    }
}

impl Cache for ApNote {
    async fn cache<C: DbRunner + Send + Sync>(&self, conn: &C) -> &Self {
        log::debug!("Checking for attachments");
        for attachment in self.attachment.multiple() {
            log::debug!("{attachment}");
            cache_content(conn, attachment.clone().try_into()).await;
        }

        log::debug!("Checking for tags");
        for tag in self.tag.multiple() {
            log::debug!("{tag}");
            cache_content(conn, tag.clone().try_into()).await;
        }

        if let Some(ephemeral) = self.ephemeral.clone() {
            if let Some(metadata_vec) = ephemeral.metadata.clone() {
                for metadata in metadata_vec {
                    if let Some(og_image) = metadata.og_image.clone() {
                        cache_content(conn, Ok(ApImage::from(og_image).into())).await;
                    }

                    if let Some(twitter_image) = metadata.twitter_image.clone() {
                        cache_content(conn, Ok(ApImage::from(twitter_image).into())).await;
                    }
                }
            }
        }

        self
    }
}

impl Cache for ApArticle {
    async fn cache<C: DbRunner + Send + Sync>(&self, conn: &C) -> &Self {
        log::debug!("Checking for attachments");
        for attachment in self.attachment.multiple() {
            log::debug!("{attachment}");
            cache_content(conn, attachment.clone().try_into()).await;
        }

        log::debug!("Checking for tags");
        for tag in self.tag.multiple() {
            log::debug!("{tag}");
            cache_content(conn, tag.clone().try_into()).await;
        }

        if let Some(ephemeral) = self.ephemeral.clone() {
            if let Some(metadata_vec) = ephemeral.metadata.clone() {
                for metadata in metadata_vec {
                    if let Some(og_image) = metadata.og_image.clone() {
                        cache_content(conn, Ok(ApImage::from(og_image).into())).await;
                    }

                    if let Some(twitter_image) = metadata.twitter_image.clone() {
                        cache_content(conn, Ok(ApImage::from(twitter_image).into())).await;
                    }
                }
            }
        }

        self
    }
}

impl Cache for ApObject {
    async fn cache<C: DbRunner + Send + Sync>(&self, conn: &C) -> &Self {
        match self {
            ApObject::Note(note) => {
                note.cache(conn).await;
            }
            ApObject::Question(question) => {
                question.cache(conn).await;
            }
            ApObject::Article(article) => {
                article.cache(conn).await;
            }
            _ => (),
        }

        self
    }
}

impl Cache for ApQuestion {
    async fn cache<C: DbRunner + Send + Sync>(&self, conn: &C) -> &Self {
        if let MaybeMultiple::Multiple(attachments) = self.attachment.clone() {
            for attachment in attachments {
                cache_content(conn, attachment.clone().try_into()).await;
            }
        }

        if let MaybeMultiple::Multiple(tags) = self.tag.clone() {
            for tag in tags {
                cache_content(conn, tag.clone().try_into()).await;
            }
        }

        if let Some(ephemeral) = self.ephemeral.clone() {
            if let Some(metadata_vec) = ephemeral.metadata.clone() {
                for metadata in metadata_vec {
                    if let Some(og_image) = metadata.og_image.clone() {
                        cache_content(conn, Ok(ApImage::from(og_image).into())).await;
                    }

                    if let Some(twitter_image) = metadata.twitter_image.clone() {
                        cache_content(conn, Ok(ApImage::from(twitter_image).into())).await;
                    }
                }
            }
        }

        self
    }
}

impl TryFrom<ApAttachment> for Cacheable {
    type Error = anyhow::Error;

    fn try_from(attachment: ApAttachment) -> Result<Self, Self::Error> {
        match attachment {
            ApAttachment::Document(document) => Ok(Cacheable::Document(document)),
            ApAttachment::Image(image) => Ok(Cacheable::Image(image)),
            _ => Err(Self::Error::msg("not cacheable")),
        }
    }
}

pub async fn prune_cache_items<C: DbRunner>(conn: &C, cutoff: DateTime<Utc>) -> Result<usize> {
    log::info!("Pruning cache items created before {cutoff}");

    // Fetch items to potentially delete
    // Capture 'cutoff' for the closure. DateTime<Utc> is Copy.
    let old_items: Vec<CacheItem> = conn
        .run(move |c: &mut PgConnection| {
            cache::table
                .filter(cache::created_at.lt(cutoff))
                .load::<CacheItem>(c)
        })
        .await
        .context("Failed to load old cache items")?;

    // pub async fn prune_cache_items(conn: Option<&Db>, cutoff: DateTime<Utc>) -> Result<usize> {
    //     log::info!("Pruning cache items created before {cutoff}");

    //     // Fetch items to potentially delete
    //     // Capture 'cutoff' for the closure. DateTime<Utc> is Copy.
    //     let old_items: Vec<CacheItem> =
    //         crate::db::run_db_op(conn, &crate::POOL, move |c: &mut PgConnection| {
    //             cache::table
    //                 .filter(cache::created_at.lt(cutoff))
    //                 .load::<CacheItem>(c)
    //         })
    //         .await
    //         .context("Failed to load old cache items")?;

    let mut deleted_count = 0;
    let mut deleted_ids = Vec::new();

    for item in old_items {
        if let Some(ref path_suffix) = item.path {
            let file_path = format!("{}/cache/{}", &*crate::MEDIA_DIR, path_suffix);

            match fs::remove_file(&file_path).await {
                Ok(_) => {
                    log::debug!("Deleted cached file: {file_path}");
                    deleted_ids.push(item.id);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    log::warn!("Cached file not found, marking for DB deletion: {file_path}");
                    deleted_ids.push(item.id);
                }
                Err(e) => {
                    log::error!("Failed to delete cached file {file_path}: {e}");
                }
            }
        } else {
            log::warn!(
                "Cache item ID {} has no path, marking for DB deletion.",
                item.id
            );
            deleted_ids.push(item.id);
        }
    }

    // Database deletion part
    if !deleted_ids.is_empty() {
        let ids_to_delete = deleted_ids.clone();
        deleted_count = conn
            .run(move |c: &mut PgConnection| {
                diesel::delete(cache::table.filter(cache::id.eq_any(ids_to_delete))).execute(c)
            })
            .await
            .context("Failed to delete cache records from database")?;
        log::info!("Deleted {deleted_count} cache records from database.");
    } else {
        // // Database deletion part
        // if !deleted_ids.is_empty() {
        //     let ids_to_delete = deleted_ids.clone();
        //     deleted_count = crate::db::run_db_op(conn, &crate::POOL, move |c: &mut PgConnection| {
        //         diesel::delete(cache::table.filter(cache::id.eq_any(ids_to_delete))).execute(c)
        //     })
        //     .await
        //     .context("Failed to delete cache records from database")?;
        //     log::info!("Deleted {deleted_count} cache records from database.");
        // } else {
        log::info!("No cache records needed database deletion.");
    }

    Ok(deleted_count)
}

impl TryFrom<ApTag> for Cacheable {
    type Error = anyhow::Error;

    fn try_from(tag: ApTag) -> Result<Self, Self::Error> {
        if let ApTag::Emoji(emoji) = tag {
            Ok(Cacheable::Image(emoji.icon))
        } else {
            Err(Self::Error::msg("not cacheable"))
        }
    }
}

impl From<ApDocument> for Cacheable {
    fn from(document: ApDocument) -> Self {
        Cacheable::Document(document)
    }
}

impl From<ApImage> for Cacheable {
    fn from(image: ApImage) -> Self {
        Cacheable::Image(image)
    }
}

impl TryFrom<Result<ApImage>> for Cacheable {
    type Error = anyhow::Error;

    fn try_from(image: Result<ApImage>) -> Result<Self, Self::Error> {
        if let Ok(image) = image {
            Ok(Cacheable::Image(image))
        } else {
            Err(Self::Error::msg("failed to convert image to Cacheable"))
        }
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = cache)]
pub struct NewCacheItem {
    pub uuid: String,
    pub url: String,
    pub media_type: Option<String>,
    pub height: Option<i32>,
    pub width: Option<i32>,
    pub blurhash: Option<String>,
    pub path: Option<String>,
}

impl NewCacheItem {
    pub async fn download<C: DbRunner + Send + Sync>(
        &self,
        conn: &C,
        profile: Option<Actor>,
    ) -> Result<Self> {
        download_image(conn, profile, self.clone()).await
    }
}

impl TryFrom<ApDocument> for NewCacheItem {
    type Error = &'static str;

    fn try_from(document: ApDocument) -> Result<Self, Self::Error> {
        let uuid = uuid::Uuid::new_v4().to_string();

        if let Some(url) = document.url {
            Ok(NewCacheItem {
                uuid,
                url,
                width: document.width,
                height: document.height,
                media_type: document.media_type,
                blurhash: document.blurhash,
                path: None,
            })
        } else {
            Err("INSUFFICIENT DATA IN DOCUMENT TO CONSTRUCT CACHE ITEM")
        }
    }
}

impl From<ApImage> for NewCacheItem {
    fn from(image: ApImage) -> Self {
        NewCacheItem {
            uuid: uuid::Uuid::new_v4().to_string(),
            url: image.url,
            width: None,
            height: None,
            media_type: image.media_type,
            blurhash: None,
            path: None,
        }
    }
}

impl Cache for ApActor {
    async fn cache<C: DbRunner + Send + Sync>(&self, conn: &C) -> &Self {
        if let MaybeMultiple::Multiple(tags) = self.tag.clone() {
            for tag in tags {
                cache_content(conn, tag.try_into()).await;
            }
        };

        for image in vec![self.image.clone(), self.icon.clone()]
            .into_iter()
            .flatten()
        {
            cache_content(conn, Ok(image.clone().into())).await;
        }

        self
    }
}

pub async fn create_cache_item<C: DbRunner + Send + Sync>(
    conn: &C,
    cache_item: NewCacheItem,
) -> Result<CacheItem> {
    conn.run(move |c: &mut PgConnection| {
        diesel::insert_into(cache::table)
            .values(&cache_item)
            .on_conflict_do_nothing()
            .get_result::<CacheItem>(c)
    })
    .await
}

pub async fn delete_cache_item_by_url<C: DbRunner + Send + Sync>(
    conn: &C,
    url: String,
) -> Result<()> {
    // 1. Find the cache item by URL
    let item_to_delete = get_cache_item_by_url(conn, url.clone())
        .await
        .map_err(|_| anyhow!("Cache item with URL '{url}' not found"))?;

    // 2. If it has a path, delete the file from disk
    if let Some(ref path_suffix) = item_to_delete.path {
        let file_path = format!("{}/cache/{path_suffix}", &*crate::MEDIA_DIR);
        log::info!("Attempting to delete file: {file_path}");
        match fs::remove_file(&file_path).await {
            Ok(_) => {
                log::info!("Successfully deleted file: {file_path}");
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::warn!("File not found, but proceeding to delete DB record: {file_path}");
            }
            Err(e) => {
                return Err(anyhow!("Failed to delete file {file_path}: {e}"))
                    .context(format!("Error deleting file for cache item URL: {url}"));
            }
        }
    } else {
        log::info!(
            "Cache item with URL '{url}' has no associated file path. Skipping file deletion."
        );
    }

    // 3. Delete the cache item from the database
    log::info!(
        "Attempting to delete database record for cache item ID: {}",
        item_to_delete.id
    );
    let item_id = item_to_delete.id; // Capture id for the closure
    conn.run(move |c: &mut PgConnection| {
        diesel::delete(cache::table.filter(cache::id.eq(item_id))).execute(c)
    })
    .await
    .map(|_| ())
    .context(format!(
        "Failed to delete cache record from database for URL: {url}"
    ))
}

pub async fn delete_cache_items_by_server_pattern<C: DbRunner>(
    conn: &C,
    server_pattern: String,
) -> Result<Vec<CacheItem>> {
    log::info!("Finding cache items matching server pattern: {server_pattern}");

    // Fetch items that match the server pattern
    let server_pattern_for_query = server_pattern.clone();
    let matching_items: Vec<CacheItem> = conn
        .run(move |c: &mut PgConnection| {
            sql_query(
                "SELECT id, created_at, updated_at, uuid, url, media_type, height, width, blurhash, path 
                 FROM cache 
                 WHERE url ~ $1"
            )
            .bind::<Text, _>(format!(r"^https://(\w+\.)*{}/.+", regex::escape(&server_pattern_for_query)))
            .load::<CacheItem>(c)
        })
        .await
        .context("Failed to load cache items matching server pattern")?;

    // pub async fn delete_cache_items_by_server_pattern(
    //     conn: Option<&Db>,
    //     server_pattern: String,
    // ) -> Result<Vec<CacheItem>> {
    //     log::info!("Finding cache items matching server pattern: {server_pattern}");

    //     // Fetch items that match the server pattern
    //     let server_pattern_for_query = server_pattern.clone();
    //     let matching_items: Vec<CacheItem> =
    //         crate::db::run_db_op(conn, &crate::POOL, move |c: &mut PgConnection| {
    //             sql_query(
    //                 "SELECT id, created_at, updated_at, uuid, url, media_type, height, width, blurhash, path
    //                  FROM cache
    //                  WHERE url ~ $1"
    //             )
    //             .bind::<Text, _>(format!(r"^https://(\w+\.)*{}/.+", regex::escape(&server_pattern_for_query)))
    //             .load::<CacheItem>(c)
    //         })
    //         .await
    //         .context("Failed to load cache items matching server pattern")?;

    let mut deleted_ids = Vec::new();

    for item in &matching_items {
        if let Some(ref path_suffix) = item.path {
            let file_path = format!("{}/cache/{}", &*crate::MEDIA_DIR, path_suffix);

            match fs::remove_file(&file_path).await {
                Ok(_) => {
                    log::debug!("Deleted cached file: {file_path}");
                    deleted_ids.push(item.id);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    log::warn!("Cached file not found, marking for DB deletion: {file_path}");
                    deleted_ids.push(item.id);
                }
                Err(e) => {
                    log::error!("Failed to delete cached file {file_path}: {e}");
                }
            }
        } else {
            log::warn!(
                "Cache item ID {} has no path, marking for DB deletion.",
                item.id
            );
            deleted_ids.push(item.id);
        }
    }

    // Database deletion part
    if !deleted_ids.is_empty() {
        let ids_to_delete = deleted_ids.clone();
        let deleted_count = conn
            .run(move |c: &mut PgConnection| {
                diesel::delete(cache::table.filter(cache::id.eq_any(ids_to_delete))).execute(c)
            })
            .await
            .context("Failed to delete cache records from database")?;
        log::info!("Deleted {deleted_count} records matching server pattern '{server_pattern}'.");
    } else {
        // // Database deletion part
        // if !deleted_ids.is_empty() {
        //     let ids_to_delete = deleted_ids.clone();
        //     let deleted_count =
        //         crate::db::run_db_op(conn, &crate::POOL, move |c: &mut PgConnection| {
        //             diesel::delete(cache::table.filter(cache::id.eq_any(ids_to_delete))).execute(c)
        //         })
        //         .await
        //         .context("Failed to delete cache records from database")?;
        //     log::info!("Deleted {deleted_count} records matching server pattern '{server_pattern}'.");
        // } else {
        log::info!("No cache records match server pattern '{server_pattern}'.");
    }

    Ok(matching_items)
}

// Enum to categorize failures from the primary download attempt (using reqwest)
#[derive(Debug)]
enum PrimaryAttemptFailure {
    Forbidden,                     // Specifically a 403 error, triggers rquest fallback
    HttpError(ReqwestStatusCode),  // Other HTTP errors
    NetworkOrOther(anyhow::Error), // Network errors or other issues from signed_get
    WrongContentType(String),      // Correct HTTP status, but not media
}

pub async fn download_image<C: DbRunner + Send + Sync>(
    conn: &C,
    profile: Option<Actor>,
    cache_item: NewCacheItem,
) -> Result<NewCacheItem> {
    const MAX_ATTEMPTS: u32 = 3; // Total number of attempts

    log::debug!("Downloading image: {}", cache_item.url);
    let signing_actor = guaranteed_actor(conn, profile).await; // Resolve actor for signing once

    // Helper to save media data from a response to a cache file
    async fn save_media_data(
        response_data: ReqwestBytes,          // Use the imported ReqwestBytes
        cache_item_for_saving: &NewCacheItem, // Use original to get UUID consistently
        url_for_log: &str,
    ) -> Result<NewCacheItem> {
        let date_folder = Utc::now().format("%Y-%m-%d").to_string();
        // Use original_cache_item.uuid to ensure consistency if cache_item was cloned/modified
        let relative_path = format!("{}/{}", date_folder, cache_item_for_saving.uuid);
        let dir_path = format!("{}/cache/{}", &*crate::MEDIA_DIR, date_folder);
        tokio::fs::create_dir_all(&dir_path)
            .await
            .context(format!("Failed to create directory {dir_path}"))?;

        let file_path = format!("{}/cache/{}", &*crate::MEDIA_DIR, relative_path);
        let mut file = File::create(&file_path)
            .await
            .context(format!("Failed to create media file {file_path}"))?;
        log::debug!("Media file created for {url_for_log}: {file_path}");

        file.write_all(&response_data)
            .await
            .context(format!("Failed to write data to media file {file_path}"))?;
        log::debug!("File written for {url_for_log}: {file_path}");

        let mut updated_cache_item = cache_item_for_saving.clone();
        updated_cache_item.path = Some(relative_path);
        Ok(updated_cache_item)
    }

    for attempt_num in 1..=MAX_ATTEMPTS {
        // --- Primary (reqwest) attempt ---
        let primary_result: Result<NewCacheItem, PrimaryAttemptFailure> = async {
            match signed_get(signing_actor.clone(), cache_item.url.clone(), true).await {
                Ok(mut response) => {
                    let status = response.status();
                    log::debug!(
                        "Attempt {}/{}: Primary signed_get response status for {}: {}",
                        attempt_num,
                        MAX_ATTEMPTS,
                        cache_item.url,
                        status
                    );
                    if status == ReqwestStatusCode::FORBIDDEN {
                        return Err(PrimaryAttemptFailure::Forbidden);
                    }
                    if !status.is_success() {
                        return Err(PrimaryAttemptFailure::HttpError(status));
                    }

                    let content_type = response
                        .headers()
                        .get(CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or_default()
                        .to_lowercase();

                    if !(content_type.starts_with("image/")
                         || content_type.starts_with("video/")
                         || content_type.starts_with("audio/")
                         || content_type.contains("*/*")
                         || content_type.is_empty())
                    {
                        log::error!(
                            "Primary signed_get for {} returned unusable media content-type: {}. Returning.",
                            cache_item.url,
                            content_type
                        );
                        return Err(PrimaryAttemptFailure::WrongContentType(content_type));
                    }
                    log::debug!(
                        "Primary signed_get for {} returned media content-type: {}. Proceeding.",
                        cache_item.url,
                        content_type
                    );
                    let data = if response
                        .headers()
                        .get("transfer-encoding")
                        .and_then(|v| v.to_str().ok())
                        .map(|v| v.contains("chunked"))
                        .unwrap_or(false)
                        || response.headers().get("content-length").is_none()
                        || content_type.starts_with("video/")
                    {
                        // Try chunked reading first
                        log::debug!("Using chunked reading for {}", cache_item.url);

                        let mut data = Vec::new();
                        let mut total_read = 0;
                        const MAX_FILE_SIZE: usize = 100 * 1024 * 1024; // 100MB limit
                        let mut chunked_success = true;

                        while let Some(chunk_result) = response.chunk().await.transpose() {
                            match chunk_result {
                                Ok(chunk) => {
                                    total_read += chunk.len();
                                    if total_read > MAX_FILE_SIZE {
                                        return Err(PrimaryAttemptFailure::NetworkOrOther(anyhow!(
                                            "File too large: {} bytes", total_read
                                        )));
                                    }
                                    data.extend_from_slice(&chunk);

                                    // Add a small delay every 1MB to be gentler on the server
                                    if total_read % (1024 * 1024) == 0 {
                                        tokio::time::sleep(TokioDuration::from_millis(50)).await;
                                    }
                                }
                                Err(e) => {
                                    log::warn!(
                                        "Chunked reading failed for {}: {}. Trying fallback with regular bytes()",
                                        cache_item.url, e
                                    );
                                    chunked_success = false;
                                    break;
                                }
                            }
                        }

                        if chunked_success {
                            log::debug!("Successfully read {} bytes via chunked reading for {}",
                                        total_read,
                                        cache_item.url);
                            ReqwestBytes::from(data)
                        } else {
                            // Fallback: make a fresh request without the enhanced headers
                            log::debug!("Attempting fallback with basic request for {}", cache_item.url);
                            let fallback_response = signed_get(signing_actor.clone(), cache_item.url.clone(),
                                                               false).await
                                .context("Failed to get fallback response")
                                .map_err(PrimaryAttemptFailure::NetworkOrOther)?;
                            fallback_response
                                .bytes()
                                .await
                                .context("Failed to get bytes from fallback response")
                                .map_err(PrimaryAttemptFailure::NetworkOrOther)?
                        }
                    } else {
                        // Use regular bytes() for smaller files with known content length
                        log::debug!("Using regular bytes reading for {}", cache_item.url);
                        response
                            .bytes()
                            .await
                            .context("Failed to get bytes from primary signed_get response")
                            .map_err(PrimaryAttemptFailure::NetworkOrOther)?
                    };
                    save_media_data(data, &cache_item, &cache_item.url)
                        .await
                        .map_err(PrimaryAttemptFailure::NetworkOrOther)
                }
                Err(e) => Err(PrimaryAttemptFailure::NetworkOrOther(
                    e.context("Primary signed_get network/other error"),
                )),
            }
        }
        .await;

        match primary_result {
            Ok(saved_item) => return Ok(saved_item), // Successfully downloaded and saved
            Err(failure_reason) => {
                let error_message_for_log = match failure_reason {
                    PrimaryAttemptFailure::Forbidden => {
                        format!(
                            "Primary attempt for {} resulted in 403 (Forbidden).",
                            cache_item.url
                        )
                    }
                    PrimaryAttemptFailure::HttpError(s) => {
                        format!(
                            "Primary attempt for {} failed with HTTP status: {}.",
                            cache_item.url, s
                        )
                    }
                    PrimaryAttemptFailure::NetworkOrOther(e) => {
                        format!("Primary attempt for {} failed: {:#}", cache_item.url, e)
                    }
                    PrimaryAttemptFailure::WrongContentType(ct) => {
                        format!(
                            "Primary attempt for {} returned non-media content-type: {}",
                            cache_item.url, ct
                        )
                    }
                };
                log::debug!(
                    "Download attempt {}/{} for {} failed: {}",
                    attempt_num,
                    MAX_ATTEMPTS,
                    cache_item.url,
                    error_message_for_log
                );
                if attempt_num < MAX_ATTEMPTS {
                    // Calculate backoff based on the current attempt number (1-indexed for exponent)
                    let backoff_duration = TokioDuration::from_secs(2u64.pow(attempt_num));
                    log::info!(
                        "Retrying download for {} in {:?}...",
                        cache_item.url,
                        backoff_duration
                    );
                    sleep(backoff_duration).await;
                }
            }
        }
    }

    // If the loop completes, all attempts have failed
    Err(anyhow!(
        "All {} download attempts failed for URL: {}",
        MAX_ATTEMPTS,
        cache_item.url
    ))
}

// I'm not sure if this is ridiculous or not, but if I use a Result here as a parameter
// I can streamline the calls from the TryFrom bits above. E.g.,
// cache_content(conn, attachment.try_into()).await;
// And the From bits just need to wrap themselves in Ok(). That seems desirable right now.
pub async fn cache_content<C: DbRunner + Send + Sync>(conn: &C, cacheable: Result<Cacheable>) {
    if let Ok(cacheable) = cacheable {
        if let Ok(cache_item) = match cacheable {
            Cacheable::Document(document) => NewCacheItem::try_from(document),
            Cacheable::Image(image) => Ok(NewCacheItem::from(image)),
        } {
            if get_cache_item_by_url(conn, cache_item.url.clone())
                .await
                .is_err()
            {
                if let Ok(cache_item) = cache_item
                    .download(
                        conn,
                        get_actor_by_username(conn, (*crate::SYSTEM_USER).clone())
                            .await
                            .ok(),
                    )
                    .await
                {
                    let _ = create_cache_item(conn, cache_item).await;
                }
            }
        }
    }
}

pub async fn get_cache_item_by_uuid<C: DbRunner + Send + Sync>(
    conn: &C,
    uuid: String,
) -> Result<CacheItem> {
    conn.run(move |c| {
        let query = cache::table.filter(cache::uuid.eq(uuid));

        query.first::<CacheItem>(c)
    })
    .await
}

pub async fn get_cache_item_by_url<C: DbRunner + Send + Sync>(
    conn: &C,
    url: String,
) -> Result<CacheItem> {
    conn.run(move |c: &mut PgConnection| {
        cache::table
            .filter(cache::url.eq(url))
            .first::<CacheItem>(c)
    })
    .await
}
