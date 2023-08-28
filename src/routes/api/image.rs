use rocket::fs::NamedFile;
use rocket::futures::stream::Stream;
use rocket::http::{ContentType, Status};
use rocket::response::stream::{ByteStream, ReaderStream};
use rocket::serde::json::Json;
use rocket::tokio::fs::File;
use rocket::Data;
use rocket::{data::ToByteUnit, get, post};
use urlencoding::decode;

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

#[get("/api/user/<username>/cache?<url>")]
pub async fn cached_image(
    signed: Signed,
    conn: Db,
    username: String,
    url: String,
) -> Result<(ContentType, NamedFile), Status> {
    //if let Signed(true, VerificationType::Local) = signed {
    if let (Some(_profile), Ok(url)) =
        (get_profile_by_username(&conn, username).await, decode(&url))
    {
        if let Some(cache) = get_cache_item_by_url(&conn, url.into()).await {
            let path = format!("{}/cache/{}", &*crate::MEDIA_DIR, cache.uuid);
            if let Some(content_type) = ContentType::parse_flexible(&cache.media_type) {
                NamedFile::open(path)
                    .await
                    .map_or(Err(Status::NoContent), |x| Ok((content_type, x)))
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
    // } else {
    //     Err(Status::NoContent)
    // }
}
