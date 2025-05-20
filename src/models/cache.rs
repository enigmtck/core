use crate::db::Db;
use crate::models::actors::guaranteed_actor;
use crate::retriever::signed_get;
use crate::schema::cache;
use anyhow::{anyhow, Context, Result}; // Add Context
use chrono::{DateTime, Utc};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::Insertable;
use diesel::{AsChangeset, Identifiable, Queryable};
use jdt_activity_pub::MaybeMultiple;
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{
    ActivityPub, ApActivity, ApActor, ApAttachment, ApCollection, ApDocument, ApImage, ApNote,
    ApObject, ApQuestion, ApTag, Collectible,
};
use rocket::tokio::time::{sleep, Duration as TokioDuration}; // Renamed to avoid conflict
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};
use tokio::fs::{self, File}; // Add fs
use tokio::io::AsyncWriteExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;


use super::actors::{get_actor_by_username, Actor};

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
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
    async fn cache(&self, conn: &Db) -> &Self;
}

pub enum Cacheable {
    Document(ApDocument),
    Image(ApImage),
}

impl Cache for ApCollection {
    async fn cache(&self, conn: &Db) -> &Self {
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
    async fn cache(&self, conn: &Db) -> &Self {
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
    async fn cache(&self, conn: &Db) -> &Self {
        match self {
            ApObject::Note(note) => {
                note.cache(conn).await;
            }
            ApObject::Question(question) => {
                question.cache(conn).await;
            }
            _ => (),
        }

        self
    }
}

impl Cache for ApQuestion {
    async fn cache(&self, conn: &Db) -> &Self {
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

pub async fn prune_cache_items(conn: Option<&Db>, cutoff: DateTime<Utc>) -> Result<usize> {
    log::info!("Pruning cache items created before {cutoff}");

    // Fetch items to potentially delete
    // Capture 'cutoff' for the closure. DateTime<Utc> is Copy.
    let old_items: Vec<CacheItem> = crate::db::run_db_op(conn, &crate::POOL, move |c: &mut PgConnection| {
        cache::table
            .filter(cache::created_at.lt(cutoff))
            .load::<CacheItem>(c)
    })
    .await
    .context("Failed to load old cache items")?;
    
    let mut deleted_count = 0;
    let mut deleted_ids = Vec::new();

    // Variables for progress reporting
    let mut main_pb: Option<ProgressBar> = None;
    let mut message_pb: Option<ProgressBar> = None;
    // Determine if running in CLI mode and if stdout is a TTY
    let show_progress = conn.is_none() && !old_items.is_empty() && atty::is(atty::Stream::Stdout);
    let _multi_progress_holder: Option<MultiProgress> = if show_progress {
        let mp = MultiProgress::new();

        let p1 = mp.add(ProgressBar::new(old_items.len() as u64));
        p1.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({percent}%)")
            .expect("Failed to create main progress bar template")
            .progress_chars("=> "));
        main_pb = Some(p1);

        let p2 = mp.add(ProgressBar::new(1)); // Length 1, as it's just for messages
        p2.set_style(ProgressStyle::default_bar()
            .template("{wide_msg}") // This will display the message across the available width
            .expect("Failed to create message progress bar template"));
        // Ensure the message bar redraws even if only the message changes
        p2.enable_steady_tick(Duration::from_millis(100)); 
        message_pb = Some(p2);
        
        Some(mp) // Keep MultiProgress alive
    } else {
        None
    };

    for item in old_items {
        let file_operation_message: String;

        if let Some(ref path_suffix) = item.path {
            let file_path = format!("{}/cache/{}", &*crate::MEDIA_DIR, path_suffix);
            file_operation_message = format!("Deleting: {file_path}");

            if let Some(p_msg) = &message_pb {
                p_msg.set_message(file_path.clone()); // Show file path on the message line
            }

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
            file_operation_message = format!("Marking Item ID {} (no path) for DB deletion.", item.id);
            if let Some(p_msg) = &message_pb {
                p_msg.set_message(format!("Processing Item ID {} (no path)", item.id));
            }
            log::warn!("Cache item ID {} has no path, marking for DB deletion.", item.id);
            deleted_ids.push(item.id);
        }
        
        // If not using progress bar, log the operation (optional, could be verbose)
        if main_pb.is_none() { // i.e. conn is Some, so not CLI
             log::debug!("{file_operation_message}");
        }

        if let Some(p_main) = &main_pb {
            p_main.inc(1);
        }
    }

    if let Some(p_msg) = &message_pb {
        p_msg.finish_and_clear(); // Remove the message line
    }
    if let Some(p_main) = &main_pb {
        p_main.finish_with_message("File deletion scan complete.");
    }

    // Database deletion part remains unchanged by indicatif
    if !deleted_ids.is_empty() {
        let ids_to_delete = deleted_ids.clone();
        deleted_count = crate::db::run_db_op(conn, &crate::POOL, move |c: &mut PgConnection| {
            diesel::delete(cache::table.filter(cache::id.eq_any(ids_to_delete)))
                .execute(c)
        })
        .await
        .context("Failed to delete cache records from database")?;
        log::info!("Deleted {deleted_count} cache records from database.");
    } else {
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
    pub async fn download(&self, conn: &Db, profile: Option<Actor>) -> Result<Self> {
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
    async fn cache(&self, conn: &Db) -> &Self {
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

pub async fn create_cache_item(conn: Option<&Db>, cache_item: NewCacheItem) -> Option<CacheItem> {
    let operation = move |c: &mut PgConnection| {
        diesel::insert_into(cache::table)
            .values(&cache_item)
            .on_conflict_do_nothing()
            .get_result::<CacheItem>(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await.ok()
}

pub async fn download_image(
    conn: &Db,
    profile: Option<Actor>,
    cache_item: NewCacheItem,
) -> Result<NewCacheItem> {
    log::debug!("Downloading image: {}", cache_item.url);

    async fn retrieve_image(
        conn: &Db,
        profile: Option<Actor>,
        cache_item: NewCacheItem,
        retries: u64,
    ) -> Result<NewCacheItem> {
        if retries == 0 {
            return Err(anyhow!("Maximum retry limit reached"));
        }

        match signed_get(
            guaranteed_actor(conn, profile.clone()).await,
            cache_item.url.clone(),
            true,
        )
        .await
        {
            Ok(response) => {
                log::debug!(
                    "Response code for {}: {}",
                    cache_item.uuid,
                    response.status()
                );

                let date_folder = Utc::now().format("%Y-%m-%d").to_string();
                let relative_path = format!("{}/{}", date_folder, cache_item.uuid);
                let dir_path = format!("{}/cache/{}", &*crate::MEDIA_DIR, date_folder); // For directory creation
                tokio::fs::create_dir_all(&dir_path).await?; // Ensure the directory exists

                // Use relative_path to construct the full file path
                let file_path = format!("{}/cache/{}", &*crate::MEDIA_DIR, relative_path);
                // Create a new file to write the downloaded image to
                let mut file = File::create(file_path.clone()).await?;

                log::debug!("File created: {file_path}");

                let data = response.bytes().await?;
                file.write_all(&data).await?;

                log::debug!("File written: {file_path}");

                let mut updated_cache_item = cache_item.clone();
                updated_cache_item.path = Some(relative_path);
                Ok(updated_cache_item)
            }
            Err(e) => {
                log::warn!(
                    "Failed to retrieve remote Image: {} | {e}",
                    cache_item.url.clone()
                );
                log::warn!("Remaining attempts: {retries}");
                let backoff = TokioDuration::from_secs(120 / retries); // Use aliased TokioDuration
                sleep(backoff).await;
                Box::pin(retrieve_image(conn, profile, cache_item, retries - 1)).await
            }
        }
    }

    retrieve_image(conn, profile, cache_item.clone(), 10).await
}

// I'm not sure if this is ridiculous or not, but if I use a Result here as a parameter
// I can streamline the calls from the TryFrom bits above. E.g.,
// cache_content(conn, attachment.try_into()).await;
// And the From bits just need to wrap themselves in Ok(). That seems desirable right now.
pub async fn cache_content(conn: &Db, cacheable: Result<Cacheable>) {
    if let Ok(cacheable) = cacheable {
        if let Ok(cache_item) = match cacheable {
            Cacheable::Document(document) => NewCacheItem::try_from(document),
            Cacheable::Image(image) => Ok(NewCacheItem::from(image)),
        } {
            if get_cache_item_by_url(conn.into(), cache_item.url.clone())
                .await
                .is_none()
            {
                if let Ok(cache_item) = cache_item
                    .download(
                        conn,
                        get_actor_by_username(conn, (*crate::SYSTEM_USER).clone()).await,
                    )
                    .await
                {
                    create_cache_item(conn.into(), cache_item).await;
                }
            }
        }
    }
}

pub async fn get_cache_item_by_uuid(conn: &Db, uuid: String) -> Option<CacheItem> {
    conn.run(move |c| {
        let query = cache::table.filter(cache::uuid.eq(uuid));

        query.first::<CacheItem>(c)
    })
    .await
    .ok()
}

pub async fn get_cache_item_by_url(conn: Option<&Db>, url: String) -> Option<CacheItem> {
    let operation = move |c: &mut PgConnection| {
        cache::table
            .filter(cache::url.eq(url))
            .first::<CacheItem>(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await.ok()
}
