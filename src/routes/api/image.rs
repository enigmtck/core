use base64::{engine::general_purpose, engine::Engine as _};
use rocket::fs::NamedFile;
use rocket::http::{ContentType, Status};
use rocket::serde::json::Json;
use rocket::Data;
use rocket::{data::ToByteUnit, get, post};

use crate::db::Db;
use crate::fairings::signatures::Signed;
use crate::models::actors::get_actor_by_username;
use crate::models::cache::{get_cache_item_by_url, Cacheable};
use jdt_activity_pub::ApAttachment;
use jdt_activity_pub::{ApDocument, ApImage, ApVideo};

#[post("/api/user/<username>/media", data = "<media>")]
pub async fn upload_media(
    signed: Signed,
    conn: Db,
    username: String,
    mut media: Data<'_>,
) -> Result<Json<ApAttachment>, Status> {
    if signed.local() {
        let _profile = get_actor_by_username(&conn, username)
            .await
            .ok_or(Status::NotFound)?;

        let header = media.peek(512).await;
        let kind = infer::get(header).ok_or(Status::UnsupportedMediaType)?;
        let filename = format!("{}.{}", uuid::Uuid::new_v4(), kind.extension());

        let path = &format!("{}/uploads", *crate::MEDIA_DIR);
        let full_path = &format!("{path}/{filename}");

        let file = media
            .open(100.mebibytes())
            .into_file(full_path)
            .await
            .map_err(|e| {
                log::error!("Failed to save file: {e:#?}");
                Status::InternalServerError
            })?;

        if file.is_complete() {
            let mime_type_str = kind.mime_type().to_string();
            let document: ApDocument = if mime_type_str.starts_with("image/") {
                let mut image_obj =
                    ApImage::initialize(path.to_string(), filename, mime_type_str.clone());
                image_obj.clean().map_err(|e| {
                    log::error!("Failed to clean ApImage ({path}): {e:?}");
                    Status::InternalServerError
                })?;
                image_obj.analyze().map_err(|e| {
                    log::error!("Failed to analyze ApImage ({path}): {e:?}");
                    Status::InternalServerError
                })?;
                ApDocument::try_from(image_obj).map_err(|e| {
                    log::error!("Failed to convert ApImage to ApDocument ({path}): {e:?}");
                    Status::InternalServerError
                })?
            } else if mime_type_str.starts_with("video/") {
                let mut video_obj =
                    ApVideo::initialize(path.to_string(), filename, mime_type_str.clone());
                video_obj.analyze().map_err(|e| {
                    log::error!("Failed to analyze ApVideo ({path}): {e:?}");
                    Status::InternalServerError
                })?;
                ApDocument::try_from(video_obj).map_err(|e| {
                    log::error!("Failed to convert ApVideo to ApDocument ({path}): {e:?}");
                    Status::InternalServerError
                })?
            } else {
                log::warn!("Unsupported media type: {mime_type_str}");
                return Err(Status::UnsupportedMediaType);
            };

            Ok(Json(document.into()))
        } else {
            log::error!("File incomplete (excessive length?)");
            Err(Status::PayloadTooLarge)
        }
    } else {
        log::error!("Bad or missing signature");
        Err(Status::Forbidden)
    }
}

// This request needs to be updated to require signing (it's effectively an open proxy)
#[get("/api/cache?<url>")]
pub async fn cached_image(conn: Db, url: String) -> Result<(ContentType, NamedFile), Status> {
    log::debug!("Cache URL before decoding: {url}");

    // Decode the URL parameter
    let decoded_url_bytes = general_purpose::URL_SAFE_NO_PAD.decode(&url).map_err(|e| {
        log::error!("Failed to decode URL: {e:#?}");
        Status::BadRequest
    })?;

    let decoded_url_string = String::from_utf8(decoded_url_bytes).map_err(|e| {
        log::error!("Failed to decode URL as UTF8: {e:#?}");
        Status::BadRequest
    })?;

    log::debug!("Decoded cache URL: {decoded_url_string}");

    // Attempt to get the item from cache, or download and cache if not found
    let cache_item = match get_cache_item_by_url(Some(&conn), decoded_url_string.clone()).await {
        Some(item) => {
            log::info!("Serving from cache: {decoded_url_string}");
            item
        }
        None => {
            log::info!("Not in cache, attempting to download and cache: {decoded_url_string}");

            // Construct ApImage and Cacheable for the runner's cache_content function
            let ap_image = ApImage::from(decoded_url_string.clone());
            let cacheable_image = Cacheable::Image(ap_image);

            // Call the refined runner's cache_content function
            match crate::runner::cache::cache_content(&conn, Ok(cacheable_image)).await {
                Ok(item) => {
                    log::info!("Successfully downloaded and cached: {decoded_url_string}");
                    item
                }
                Err(e) => {
                    log::error!("Failed to download/cache image {decoded_url_string}: {e:#?}");
                    // Depending on the error from cache_content, Status::NotFound might also be appropriate
                    return Err(Status::InternalServerError);
                }
            }
        }
    };

    // Logic to serve the file from the (now definitely existing or error-ed out) cache_item
    let path_suffix = cache_item
        .path
        .as_deref()
        .unwrap_or(cache_item.uuid.as_str()); // Fallback to UUID if path is somehow None

    let path = format!("{}/cache/{}", &*crate::MEDIA_DIR, path_suffix);

    let media_type_str = cache_item
        .media_type
        .as_deref()
        .unwrap_or("application/octet-stream"); // Default if media_type is None

    // Use unwrap_or_else for ContentType to provide a default and log a warning
    let content_type = ContentType::parse_flexible(media_type_str).unwrap_or_else(|| {
        log::warn!(
            "Failed to parse media_type '{media_type_str}' for {path}, defaulting to application/octet-stream"
        );
        ContentType::new("application", "octet-stream")
    });

    NamedFile::open(&path).await.map_or_else(
        |e| {
            log::error!(
                "Failed to open cached file '{path}': {e:#?}. Cache item details: {cache_item:?}"
            );
            Err(Status::InternalServerError)
        },
        |named_file| Ok((content_type, named_file)),
    )
}
