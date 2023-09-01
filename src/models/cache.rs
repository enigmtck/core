use crate::activity_pub::retriever::maybe_signed_get;
use crate::activity_pub::{ApAttachment, ApDocument, ApImage, ApTag};
use crate::db::Db;
use crate::models::profiles::get_profile_by_username;
use crate::schema::cache;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use super::profiles::Profile;

async fn download_image(
    profile: Option<Profile>,
    cache_item: NewCacheItem,
) -> Option<NewCacheItem> {
    log::debug!("DOWNLOADING IMAGE: {}", cache_item.url);

    let path = format!("{}/cache/{}", &*crate::MEDIA_DIR, cache_item.uuid);
    // Send an HTTP GET request to the URL
    if let Ok(response) = maybe_signed_get(profile, cache_item.url.clone(), true).await {
        //if let Ok(response) = reqwest::get(&cache_item.url).await {
        // Create a new file to write the downloaded image to
        if let Ok(mut file) = File::create(path).await {
            if let Ok(data) = response.bytes().await {
                if file.write_all(&data).await.is_ok() {
                    Some(cache_item)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub trait Cache {
    async fn cache(&self, conn: &Db) -> &Self;
}

pub enum Cacheable {
    Document(ApDocument),
    Image(ApImage),
}

impl TryFrom<ApAttachment> for Cacheable {
    type Error = &'static str;

    fn try_from(attachment: ApAttachment) -> Result<Self, Self::Error> {
        if let ApAttachment::Document(document) = attachment {
            Ok(Cacheable::Document(document))
        } else {
            Err("NOT CACHEABLE")
        }
    }
}

impl TryFrom<ApTag> for Cacheable {
    type Error = &'static str;

    fn try_from(tag: ApTag) -> Result<Self, Self::Error> {
        if let ApTag::Emoji(emoji) = tag {
            Ok(Cacheable::Image(emoji.icon))
        } else {
            Err("NOT CACHEABLE")
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

// I'm not sure if this is ridiculous or not, but if I use a Result here as a parameter
// I can streamline the calls from the TryFrom bits above. E.g.,
// cache_content(conn, attachment.try_into()).await;
// And the From bits just need to wrap themselves in Ok(). That seems desirable right now.
pub async fn cache_content(conn: &Db, cacheable: Result<Cacheable, &str>) {
    if let Ok(cacheable) = cacheable {
        if let Ok(cache_item) = match cacheable {
            Cacheable::Document(document) => NewCacheItem::try_from(document),
            Cacheable::Image(image) => Ok(NewCacheItem::from(image)),
        } {
            if get_cache_item_by_url(conn, cache_item.url.clone())
                .await
                .is_none()
            {
                if let Some(cache_item) = cache_item
                    .download(get_profile_by_username(conn, "justin".to_string()).await)
                    .await
                {
                    create_cache_item(conn, cache_item).await;
                }
            }
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
}

impl NewCacheItem {
    pub async fn download(&self, profile: Option<Profile>) -> Option<Self> {
        download_image(profile, self.clone()).await
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
        }
    }
}

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
}

async fn create_cache_item(conn: &Db, cache_item: NewCacheItem) -> Option<CacheItem> {
    conn.run(move |c| {
        diesel::insert_into(cache::table)
            .values(&cache_item)
            .on_conflict_do_nothing()
            .get_result::<CacheItem>(c)
    })
    .await
    .ok()
}

pub async fn get_cache_item_by_uuid(conn: &Db, uuid: String) -> Option<CacheItem> {
    conn.run(move |c| {
        let query = cache::table.filter(cache::uuid.eq(uuid));

        query.first::<CacheItem>(c)
    })
    .await
    .ok()
}

pub async fn get_cache_item_by_url(conn: &Db, url: String) -> Option<CacheItem> {
    conn.run(move |c| {
        let query = cache::table.filter(cache::url.eq(url));

        query.first::<CacheItem>(c)
    })
    .await
    .ok()
}
