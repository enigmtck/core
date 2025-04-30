use base64::{engine::general_purpose, engine::Engine as _};
use rocket::fs::NamedFile;
use rocket::http::{ContentType, Status};
use rocket::serde::json::Json;
use rocket::Data;
use rocket::{data::ToByteUnit, get, post};

use crate::db::Db;
use crate::fairings::signatures::Signed;
use crate::models::actors::get_actor_by_username;
use crate::models::cache::get_cache_item_by_url;
use jdt_activity_pub::ApAttachment;

#[post("/api/user/<username>/image", data = "<media>")]
pub async fn upload_image(
    signed: Signed,
    conn: Db,
    username: String,
    media: Data<'_>,
) -> Result<Json<ApAttachment>, Status> {
    if signed.local() {
        let _profile = get_actor_by_username(&conn, username)
            .await
            .ok_or(Status::NotFound)?;

        let filename = uuid::Uuid::new_v4();
        let path = &format!("{}/uploads/{}", *crate::MEDIA_DIR, filename);

        let file = media
            .open(10.mebibytes())
            .into_file(path)
            .await
            .map_err(|e| {
                log::error!("FAILED TO SAVE FILE: {e:#?}");
                Status::InternalServerError
            })?;

        if file.is_complete() {
            if let Ok(attachment) = ApAttachment::try_from(filename.to_string()) {
                Ok(Json(attachment))
            } else {
                log::error!("FAILED TO CREATE ATTACHMENT");
                Err(Status::NotAcceptable)
            }
        } else {
            log::error!("FILE INCOMPLETE");
            Err(Status::PayloadTooLarge)
        }
    } else {
        log::error!("BAD SIGNATURE");
        Err(Status::Forbidden)
    }
}

#[get("/api/cache?<url>")]
pub async fn cached_image(conn: Db, url: String) -> Result<(ContentType, NamedFile), Status> {
    log::debug!("CACHE URL BEFORE DECODING: {url}");

    let url = general_purpose::URL_SAFE_NO_PAD.decode(url).map_err(|e| {
        log::error!("FAILED TO DECODE url: {e:#?}");
        Status::BadRequest
    })?;

    let url = String::from_utf8(url).map_err(|e| {
        log::error!("FAILED TO DECODE url: {e:#?}");
        Status::BadRequest
    })?;

    log::debug!("DECODED CACHE URL: {url}");
    let cache = get_cache_item_by_url((&conn).into(), url)
        .await
        .ok_or(Status::NotFound)?;

    // Borrow path or uuid without cloning
    let path_suffix = cache.path.as_deref().unwrap_or(cache.uuid.as_str());
    let path = format!("{}/cache/{}", &*crate::MEDIA_DIR, path_suffix);
    // Borrow media_type without cloning cache or allocating "any"
    let media_type_str = cache.media_type.as_deref().unwrap_or("any");

    let content_type = ContentType::parse_flexible(media_type_str).ok_or_else(|| {
        log::error!("Failed to determine ContentType");
        Status::InternalServerError
    })?;

    NamedFile::open(path).await.map_or_else(
        |e| {
            log::error!("Failed to open file: {e:#?}");
            Err(Status::InternalServerError)
        },
        |x| Ok((content_type, x)),
    )
}
