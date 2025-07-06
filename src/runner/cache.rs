use crate::db::Db;
// Ensure this is imported if SYSTEM_USER is used for downloads
use crate::models::cache::{
    create_cache_item, get_cache_item_by_url, CacheItem, Cacheable, NewCacheItem,
};
use anyhow::{anyhow, Result}; // Ensure anyhow is imported

pub async fn cache_content(conn: &Db, cacheable: Result<Cacheable>) -> Result<CacheItem> {
    // Convert Cacheable to NewCacheItem. Propagate errors from try_from/from.
    let new_cache_item_to_process = match cacheable? {
        Cacheable::Document(document) => {
            NewCacheItem::try_from(document).map_err(anyhow::Error::msg)
        }
        Cacheable::Image(image) => Ok(NewCacheItem::from(image)),
    }?;

    let item_url = new_cache_item_to_process.url.clone();

    // 1. Attempt to get it first. This handles race conditions or if it was cached by another process.
    if let Ok(existing_item) = get_cache_item_by_url(conn, item_url.clone()).await {
        log::debug!("Item found in cache (runner::cache::cache_content): {item_url}");
        return Ok(existing_item);
    }

    // 2. If not cached, download it.
    //    The download method in models/cache.rs uses guaranteed_actor, which might fetch SYSTEM_USER.
    //    Passing None for profile to download() will make it use SYSTEM_USER.
    log::debug!("Item not in cache, attempting download: {item_url}");
    let downloaded_item_meta = new_cache_item_to_process.download(conn, None).await?; // Pass conn, profile is None

    // 3. Create the item in the database.
    //    create_cache_item uses ON CONFLICT DO NOTHING.
    //    The result of create_cache_item (Result<CacheItem>) is not strictly needed here
    //    because we will re-fetch.
    let _ = create_cache_item(conn, downloaded_item_meta.clone()).await;

    // 4. Re-fetch the item to ensure we get the definitive version from the DB.
    //    This covers cases where it was just inserted or inserted by a racing process.
    get_cache_item_by_url(conn, item_url.clone())
        .await
        .map_err(|_| {
            anyhow!(format!(
            "Failed to retrieve cache item from DB after download and create attempt for URL: {}",
            item_url
        ))
        })
}

// pub async fn cache_content(conn: &Db, cacheable: Result<Cacheable>) -> Result<CacheItem> {
//     // Convert Cacheable to NewCacheItem. Propagate errors from try_from/from.
//     let new_cache_item_to_process = match cacheable? {
//         Cacheable::Document(document) => {
//             NewCacheItem::try_from(document).map_err(anyhow::Error::msg)
//         }
//         Cacheable::Image(image) => Ok(NewCacheItem::from(image)),
//     }?;

//     let item_url = new_cache_item_to_process.url.clone();

//     // 1. Attempt to get it first. This handles race conditions or if it was cached by another process.
//     if let Some(existing_item) = get_cache_item_by_url(Some(conn), item_url.clone()).await {
//         log::debug!("Item found in cache (runner::cache::cache_content): {item_url}");
//         return Ok(existing_item);
//     }

//     // 2. If not cached, download it.
//     //    The download method in models/cache.rs uses guaranteed_actor, which might fetch SYSTEM_USER.
//     //    Passing None for profile to download() will make it use SYSTEM_USER.
//     log::debug!("Item not in cache, attempting download: {item_url}");
//     let downloaded_item_meta = new_cache_item_to_process.download(conn, None).await?; // Pass conn, profile is None

//     // 3. Create the item in the database.
//     //    create_cache_item uses ON CONFLICT DO NOTHING.
//     //    The result of create_cache_item (Option<CacheItem>) is not strictly needed here
//     //    because we will re-fetch.
//     create_cache_item(Some(conn), downloaded_item_meta.clone()).await;

//     // 4. Re-fetch the item to ensure we get the definitive version from the DB.
//     //    This covers cases where it was just inserted or inserted by a racing process.
//     get_cache_item_by_url(Some(conn), item_url.clone())
//         .await
//         .ok_or_else(|| {
//             anyhow!(
//                 "Failed to retrieve cache item from DB after download and create attempt for URL: {}",
//                 item_url
//             )
//         })
// }
