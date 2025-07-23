use crate::{
    models::objects::get_object_by_uuid,
    server::{extractors::AxumSigned, AppState},
};
use axum::extract::{Query, State};
use jdt_activity_pub::ApObject;
use reqwest::StatusCode;
use serde::Deserialize;

use super::ActivityJson;

#[derive(Deserialize)]
pub struct ObjectQuery {
    uuid: String,
}

pub async fn object_get(
    State(state): State<AppState>,
    _signed: AxumSigned,
    Query(query): Query<ObjectQuery>,
) -> Result<ActivityJson<ApObject>, StatusCode> {
    let conn = match state.db_pool.get().await {
        Ok(c) => c,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    get_object_by_uuid(&conn, query.uuid)
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
