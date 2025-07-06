use crate::{db::Db, models::objects::get_object_by_uuid};
use jdt_activity_pub::ApObject;
use rocket::{get, http::Status, serde::json::Json};

use super::ActivityJson;

#[get("/objects/<uuid>")]
pub async fn object_get(conn: Db, uuid: String) -> Result<ActivityJson<ApObject>, Status> {
    get_object_by_uuid(&conn, uuid)
        .await
        .map_err(|e| {
            log::error!("UNABLE TO RETRIEVE OBJECT: {e:#?}");
            Status::InternalServerError
        })
        .and_then(|x| match ApObject::try_from(x) {
            Ok(ap_object) => Ok(ActivityJson(Json(ap_object))),
            Err(e) => {
                log::error!("UNABLE TO CONVERT TO ApObject: {e:#?}");
                Err(Status::InternalServerError)
            }
        })
}
