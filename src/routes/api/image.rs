use base64::{engine::general_purpose, engine::Engine as _};
use rocket::fs::NamedFile;
use rocket::http::{ContentType, Status};
use rocket::serde::json::Json;
use rocket::Data;
use rocket::{data::ToByteUnit, get, post};

use crate::activity_pub::ApAttachment;
use crate::db::Db;
use crate::fairings::signatures::Signed;
use crate::models::cache::get_cache_item_by_url;
use crate::models::profiles::get_profile_by_username;
use crate::signing::VerificationType;

#[post("/api/user/<username>/image", data = "<media>")]
pub async fn upload_image(
    signed: Signed,
    conn: Db,
    username: String,
    media: Data<'_>,
) -> Result<Json<ApAttachment>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Some(_profile) = get_profile_by_username(&conn, username).await {
            let filename = uuid::Uuid::new_v4();
            let path = &format!("{}/uploads/{}", *crate::MEDIA_DIR, filename);

            if let Ok(file) = media.open(10.mebibytes()).into_file(path).await {
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
                log::error!("FAILED TO OPEN MEDIA");
                Err(Status::InternalServerError)
            }
        } else {
            log::error!("FAILED TO RETRIEVE PROFILE");
            Err(Status::NotFound)
        }
    } else {
        log::error!("BAD SIGNATURE");
        Err(Status::Forbidden)
    }
}

#[get("/api/cache?<url>")]
pub async fn cached_image(conn: Db, url: String) -> Result<(ContentType, NamedFile), Status> {
    log::debug!("CACHE URL BEFORE DECODING: {url}");

    if let Ok(url) = urlencoding::decode(&url) {
        if let Ok(url) = general_purpose::STANDARD_NO_PAD.decode(url.to_string()) {
            if let Ok(url) = String::from_utf8(url) {
                log::debug!("DECODED CACHE URL: {url}");
                if let Some(cache) = get_cache_item_by_url(&conn, url).await {
                    let path = format!("{}/cache/{}", &*crate::MEDIA_DIR, cache.uuid);
                    let media_type = &cache.clone().media_type.map_or("any".to_string(), |x| x);

                    if let Some(content_type) = ContentType::parse_flexible(media_type) {
                        NamedFile::open(path)
                            .await
                            .map_or(Err(Status::InternalServerError), |x| Ok((content_type, x)))
                    } else {
                        Err(Status::InternalServerError)
                    }
                } else {
                    Err(Status::NotFound)
                }
            } else {
                Err(Status::BadRequest)
            }
        } else {
            Err(Status::BadRequest)
        }
    } else {
        Err(Status::BadRequest)
    }
}
