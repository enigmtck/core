use crate::{activity_pub::ApNote, db::Db, models::notes::get_note_by_uuid};
use rocket::{get, http::Status, serde::json::Json};

#[get("/notes/<uuid>")]
pub async fn note_get(conn: Db, uuid: String) -> Result<Json<ApNote>, Status> {
    if let Some(x) = get_note_by_uuid(&conn, uuid).await {
        Ok(Json(x.into()))
    } else {
        Err(Status::NoContent)
    }
}
