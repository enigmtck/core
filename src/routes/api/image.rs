use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{
    data::{Data, ToByteUnit},
    post,
};

use crate::activity_pub::ApAttachment;
use crate::db::Db;
use crate::fairings::signatures::Signed;
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
                        Err(Status::NoContent)
                    }
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::Forbidden)
    }
}
