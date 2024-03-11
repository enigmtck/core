use crate::activity_pub::retriever::signed_get;
use crate::activity_pub::{ApAttachment, ApDocument, ApImage, ApTag};
use crate::db::Db;
use crate::models::profiles::{get_profile_by_username, guaranteed_profile};
use crate::schema::cache;
use crate::POOL;
use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use super::profiles::Profile;

async fn download_image(
    conn: Option<&Db>,
    profile: Option<Profile>,
    cache_item: NewCacheItem,
) -> Result<NewCacheItem> {
    log::debug!("DOWNLOADING IMAGE: {}", cache_item.url);

    let path = format!("{}/cache/{}", &*crate::MEDIA_DIR, cache_item.uuid);

    // Send an HTTP GET request to the URL
    let response = signed_get(
        guaranteed_profile(conn, profile).await,
        cache_item.url.clone(),
        true,
    )
    .await?;

    log::debug!(
        "RESPONSE CODE FOR {}: {}",
        cache_item.uuid,
        response.status()
    );

    // Create a new file to write the downloaded image to
    let mut file = File::create(path.clone()).await?;

    log::debug!("FILE CREATED: {path}");

    let data = response.bytes().await?;
    file.write_all(&data).await?;

    log::debug!("FILE WRITTEN: {path}");

    Ok(cache_item)
}

pub trait Cache {
    async fn cache(&self, conn: &Db) -> &Self;
}

pub enum Cacheable {
    Document(ApDocument),
    Image(ApImage),
}

impl TryFrom<ApAttachment> for Cacheable {
    type Error = anyhow::Error;

    fn try_from(attachment: ApAttachment) -> Result<Self, Self::Error> {
        if let ApAttachment::Document(document) = attachment {
            Ok(Cacheable::Document(document))
        } else {
            Err(Self::Error::msg("not cacheable"))
        }
    }
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
                        conn.into(),
                        get_profile_by_username(conn.into(), (*crate::SYSTEM_USER).clone()).await,
                    )
                    .await
                {
                    create_cache_item(conn.into(), cache_item).await;
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
    pub async fn download(&self, conn: Option<&Db>, profile: Option<Profile>) -> Result<Self> {
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub uuid: String,
    pub url: String,
    pub media_type: Option<String>,
    pub height: Option<i32>,
    pub width: Option<i32>,
    pub blurhash: Option<String>,
}

pub async fn create_cache_item(conn: Option<&Db>, cache_item: NewCacheItem) -> Option<CacheItem> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(cache::table)
                    .values(&cache_item)
                    .on_conflict_do_nothing()
                    .get_result::<CacheItem>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(cache::table)
                .values(&cache_item)
                .on_conflict_do_nothing()
                .get_result::<CacheItem>(&mut pool)
                .ok()
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
    match conn {
        Some(conn) => conn
            .run(move |c| {
                cache::table
                    .filter(cache::url.eq(url))
                    .first::<CacheItem>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            cache::table
                .filter(cache::url.eq(url))
                .first::<CacheItem>(&mut pool)
                .ok()
        }
    }
}
