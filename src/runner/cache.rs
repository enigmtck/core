use super::POOL;
use crate::models::cache::{CacheItem, NewCacheItem};
use crate::schema::cache;
use diesel::prelude::*;

pub fn create_cache_item(cache_item: NewCacheItem) -> Option<CacheItem> {
    if let Ok(mut conn) = POOL.get() {
        diesel::insert_into(cache::table)
            .values(&cache_item)
            .get_result::<CacheItem>(&mut conn)
            .ok()
    } else {
        None
    }
}
