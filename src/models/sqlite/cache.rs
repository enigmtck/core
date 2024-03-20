use crate::db::Db;
use crate::models::cache::NewCacheItem;
use crate::schema::cache;
use crate::POOL;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::Serialize;

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
                    .execute(c)?;

                cache::table.order(cache::id.desc()).first::<CacheItem>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(cache::table)
                .values(&cache_item)
                .on_conflict_do_nothing()
                .execute(&mut pool)
                .ok()?;

            cache::table
                .order(cache::id.desc())
                .first::<CacheItem>(&mut pool)
                .ok()
        }
    }
}
