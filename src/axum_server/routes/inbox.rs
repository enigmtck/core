use crate::axum_server::extractors::AxumSigned;
use crate::axum_server::AppState;
use crate::db::Db;
use crate::routes::{
    inbox::{shared_inbox_get, InboxView},
    ActivityJson,
};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json as AxumJson;
use deadpool_diesel::Connection;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct InboxQuery {
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub limit: u8,
    pub view: Option<InboxView>,
    pub hashtags: Option<Vec<String>>,
}

pub async fn axum_shared_inbox_get(
    State(app_state): State<AppState>,
    AxumSigned(signed): AxumSigned,
    Query(params): Query<InboxQuery>,
) -> Response {
    let conn = match app_state.db_pool.get().await {
        Ok(conn) => conn,
        Err(e) => {
            log::error!("Failed to get DB connection from pool: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database unavailable").into_response();
        }
    };

    let db_conn: Db = Connection(conn);

    let result = shared_inbox_get(
        signed,
        db_conn,
        params.min,
        params.max,
        params.limit,
        params.view,
        params.hashtags,
    )
    .await;

    match result {
        Ok(ActivityJson(data)) => (StatusCode::OK, AxumJson(*data)).into_response(),
        Err(status) => {
            let code =
                StatusCode::from_u16(status.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            (code, status.reason().unwrap_or("").to_string()).into_response()
        }
    }
}
