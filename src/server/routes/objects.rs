use crate::{
    models::objects::get_object_by_uuid,
    server::{extractors::AxumSigned, AppState},
};
use axum::extract::{Path, State};
use jdt_activity_pub::ApObject;
use reqwest::StatusCode;

use super::ActivityJson;

pub async fn object_get(
    State(state): State<AppState>,
    _signed: AxumSigned,
    Path(uuid): Path<String>,
) -> Result<ActivityJson<ApObject>, StatusCode> {
    let conn = match state.db_pool.get().await {
        Ok(c) => c,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    log::debug!("Retrieving Object: {uuid}");

    get_object_by_uuid(&conn, uuid)
        .await
        .map_err(|e| {
            log::error!("Unable to retrieve Object: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
        .and_then(|x| match ApObject::try_from(x) {
            Ok(ap_object) => Ok(ActivityJson(ap_object)),
            Err(e) => {
                log::error!("Unable to convert to ApObject: {e:#?}");
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        })
}
