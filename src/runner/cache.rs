use super::user::get_profile_by_username;
use super::POOL;
use crate::models::cache::{CacheItem, Cacheable, NewCacheItem};
use crate::schema::cache;
use diesel::prelude::*;

pub async fn cache_content(cacheable: Cacheable) {
    if let Ok(cache_item) = match cacheable {
        Cacheable::Document(document) => NewCacheItem::try_from(document),
        Cacheable::Image(image) => NewCacheItem::try_from(image),
    } {
        if get_cache_item_by_url(cache_item.url.clone()).is_none() {
            cache_item
                .download(get_profile_by_username("justin".to_string()))
                .await
                .map(create_cache_item);
        }
    }
}

fn create_cache_item(cache_item: NewCacheItem) -> Option<CacheItem> {
    if let Ok(mut conn) = POOL.get() {
        diesel::insert_into(cache::table)
            .values(&cache_item)
            .on_conflict_do_nothing()
            .get_result::<CacheItem>(&mut conn)
            .ok()
    } else {
        None
    }
}

fn get_cache_item_by_url(url: String) -> Option<CacheItem> {
    if let Ok(mut conn) = POOL.get() {
        cache::table
            .filter(cache::url.eq(url))
            .first::<CacheItem>(&mut conn)
            .ok()
    } else {
        None
    }
}
