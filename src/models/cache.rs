use crate::activity_pub::{ApDocument, ApImage};
use crate::db::Db;
use crate::schema::cache;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

async fn download_image(cache_item: NewCacheItem) -> Option<NewCacheItem> {
    log::debug!("DOWNLOADING IMAGE: {}", cache_item.url);

    let path = format!("{}/cache/{}", &*crate::MEDIA_DIR, cache_item.uuid);
    // Send an HTTP GET request to the URL
    if let Ok(response) = reqwest::get(&cache_item.url).await {
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

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = cache)]
pub struct NewCacheItem {
    pub uuid: String,
    pub url: String,
    pub media_type: String,
    pub height: Option<i32>,
    pub width: Option<i32>,
    pub blurhash: Option<String>,
}

impl NewCacheItem {
    pub async fn download(&self) -> Option<Self> {
        download_image(self.clone()).await
    }
}

impl TryFrom<ApDocument> for NewCacheItem {
    type Error = &'static str;

    fn try_from(document: ApDocument) -> Result<Self, Self::Error> {
        let uuid = uuid::Uuid::new_v4().to_string();

        if let (Some(media_type), Some(url)) = (document.media_type, document.url) {
            Ok(NewCacheItem {
                uuid,
                url,
                width: document.width,
                height: document.height,
                media_type,
                blurhash: document.blurhash,
            })
        } else {
            Err("INSUFFICIENT DATA IN DOCUMENT TO CONSTRUCT CACHE ITEM")
        }
    }
}

impl TryFrom<ApImage> for NewCacheItem {
    type Error = &'static str;

    fn try_from(image: ApImage) -> Result<Self, Self::Error> {
        let uuid = uuid::Uuid::new_v4().to_string();

        if let Some(media_type) = image.media_type {
            Ok(NewCacheItem {
                uuid,
                url: image.url,
                width: None,
                height: None,
                media_type,
                blurhash: None,
            })
        } else {
            Err("INSUFFICIENT DATA IN IMAGE TO CONSTRUCT CACHE ITEM")
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
    pub media_type: String,
    pub height: Option<i32>,
    pub width: Option<i32>,
    pub blurhash: Option<String>,
}

pub async fn create_cache_item(conn: &Db, cache_item: NewCacheItem) -> Option<CacheItem> {
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
