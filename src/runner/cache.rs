use crate::models::cache::{
    create_cache_item, get_cache_item_by_url, CacheItem, Cacheable, NewCacheItem,
};
use crate::POOL;
use anyhow::Result;

pub async fn cache_content(cacheable: Result<Cacheable>) -> Result<Option<CacheItem>> {
    if let Ok(cache_item) = match cacheable? {
        Cacheable::Document(document) => NewCacheItem::try_from(document),
        Cacheable::Image(image) => Ok(NewCacheItem::from(image)),
    } {
        if get_cache_item_by_url(
            POOL.get()
                .expect("failed to get database connection")
                .into(),
            cache_item.url.clone(),
        )
        .await
        .is_none()
        {
            let item = cache_item.download(POOL.get()?.into(), None).await?;
            Ok(create_cache_item(POOL.get()?.into(), item).await)
        } else {
            Err(anyhow::Error::msg("failed to cache item"))
        }
    } else {
        Err(anyhow::Error::msg("failed to cache item"))
    }
}
