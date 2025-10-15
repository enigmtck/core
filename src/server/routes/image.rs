use crate::{
    models::{
        actors::get_actor_by_username,
        cache::{get_cache_item_by_url, Cacheable},
    },
    server::{extractors::AxumSigned, AppState},
};
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
};
use base64::{engine::general_purpose, Engine as _};
use jdt_activity_pub::{ApAttachment, ApDocument, ApImage, ApVideo};
use serde::Deserialize;
use tokio::fs;

pub async fn upload_media(
    signed: AxumSigned,
    State(state): State<AppState>,
    Path(username): Path<String>,
    bytes: Bytes,
) -> Result<Json<ApAttachment>, StatusCode> {
    if !signed.local() {
        return Err(StatusCode::FORBIDDEN);
    }

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    get_actor_by_username(&conn, username)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    if bytes.len() > 100 * 1024 * 1024 {
        // 100 MiB limit from Rocket version
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let kind = infer::get(&bytes).ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;
    let filename = format!("{}.{}", uuid::Uuid::new_v4(), kind.extension());

    let path = &format!("{}/uploads", *crate::MEDIA_DIR);
    let full_path = &format!("{path}/{filename}");

    fs::write(full_path, &bytes).await.map_err(|e| {
        log::error!("Failed to save file: {e:#?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mime_type_str = kind.mime_type().to_string();
    let document: ApDocument = if mime_type_str.starts_with("image/") {
        let mut image_obj = ApImage::initialize(path.to_string(), filename, mime_type_str.clone());
        image_obj.clean().map_err(|e| {
            log::error!("Failed to clean ApImage ({path}): {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        image_obj.analyze().map_err(|e| {
            log::error!("Failed to analyze ApImage ({path}): {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        ApDocument::try_from(image_obj).map_err(|e| {
            log::error!("Failed to convert ApImage to ApDocument ({path}): {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else if mime_type_str.starts_with("video/") {
        let mut video_obj = ApVideo::initialize(path.to_string(), filename, mime_type_str.clone());
        video_obj.analyze().map_err(|e| {
            log::error!("Failed to analyze ApVideo ({path}): {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        ApDocument::try_from(video_obj).map_err(|e| {
            log::error!("Failed to convert ApVideo to ApDocument ({path}): {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else {
        log::warn!("Unsupported media type: {mime_type_str}");
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    };

    Ok(Json(document.into()))
}

#[derive(Deserialize)]
pub struct CacheQuery {
    url: String,
}

pub async fn cached_image(
    State(state): State<AppState>,
    Query(query): Query<CacheQuery>,
) -> Response {
    log::debug!("Cache URL before decoding: {}", query.url);

    let decoded_url_bytes = match general_purpose::URL_SAFE_NO_PAD.decode(&query.url) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::error!("Failed to decode URL: {e:#?}");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    let decoded_url_string = match String::from_utf8(decoded_url_bytes) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to decode URL as UTF8: {e:#?}");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    log::debug!("Decoded cache URL: {decoded_url_string}");

    let conn = match state.db_pool.get().await {
        Ok(c) => c,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let cache_item = match get_cache_item_by_url(&conn, decoded_url_string.clone()).await {
        Ok(item) => {
            log::info!("Serving from cache: {decoded_url_string}");
            item
        }
        _ => {
            log::info!("Not in cache, attempting to download and cache: {decoded_url_string}");

            let ap_image = ApImage::from(decoded_url_string.clone());
            let cacheable_image = Cacheable::Image(ap_image);

            match crate::runner::cache::cache_content(&conn, Ok(cacheable_image)).await {
                Ok(item) => {
                    log::info!("Successfully downloaded and cached: {decoded_url_string}");
                    item
                }
                Err(e) => {
                    log::error!("Failed to download/cache image {decoded_url_string}: {e:#?}");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            }
        }
    };

    let path_suffix = cache_item
        .path
        .as_deref()
        .unwrap_or(cache_item.uuid.as_str());

    let path = format!("{}/cache/{}", &*crate::MEDIA_DIR, path_suffix);

    let media_type_str = cache_item
        .media_type
        .as_deref()
        .unwrap_or("application/octet-stream");

    let content_type = match media_type_str.parse() {
        Ok(ct) => ct,
        Err(_) => {
            log::warn!(
                "Failed to parse media_type '{media_type_str}' for {path}, defaulting to application/octet-stream"
            );
            header::HeaderValue::from_static("application/octet-stream")
        }
    };

    match fs::read(&path).await {
        Ok(data) => {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, content_type);
            (headers, data).into_response()
        }
        Err(e) => {
            log::error!(
                "Failed to open cached file '{path}': {e:#?}. Cache item details: {cache_item:?}"
            );
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
