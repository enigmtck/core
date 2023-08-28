use crate::activity_pub::ApDocument;
use crate::db::Db;
use crate::schema::cache;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = cache)]
pub struct NewCacheItem {
    pub uuid: String,
    pub url: String,
    pub media_type: String,
    pub height: i32,
    pub width: i32,
    pub blurhash: Option<String>,
}

impl TryFrom<ApDocument> for NewCacheItem {
    type Error = &'static str;

    fn try_from(document: ApDocument) -> Result<Self, Self::Error> {
        let uuid = uuid::Uuid::new_v4().to_string();

        if let (Some(media_type), Some(url), Some(width), Some(height)) = (
            document.media_type,
            document.url,
            document.width,
            document.height,
        ) {
            Ok(NewCacheItem {
                uuid,
                url,
                width,
                height,
                media_type,
                blurhash: document.blurhash,
            })
        } else {
            Err("INSUFFICIENT DATA IN DOCUMENT TO CONSTRUCT CACHE ITEM")
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
    pub height: i32,
    pub width: i32,
    pub blurhash: Option<String>,
}

pub async fn create_cache_item(conn: &Db, cache_item: NewCacheItem) -> Option<CacheItem> {
    conn.run(move |c| {
        diesel::insert_into(cache::table)
            .values(&cache_item)
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
