use crate::{
    axum_server::{extractors::AxumSigned, AppState},
    models::{
        activities::{TimelineFilters, TimelineView},
        actors::get_actor_by_username,
        unprocessable::create_unprocessable,
    },
    routes::{retrieve, ActivityJson, Outbox},
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use jdt_activity_pub::{ActivityPub, ApActivity, ApObject};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct OutboxQuery {
    min: Option<i64>,
    max: Option<i64>,
    limit: Option<u8>,
    page: Option<bool>,
}

pub async fn axum_outbox_get(
    State(state): State<AppState>,
    signed: AxumSigned,
    Path(username): Path<String>,
    Query(query): Query<OutboxQuery>,
) -> Result<Json<ApObject>, StatusCode> {
    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let profile = signed.profile();
    let server_url = format!("https://{}", *crate::SERVER_NAME);
    let limit = query.limit.unwrap_or(10);
    let page = query.page.unwrap_or_default();

    let base_url = format!("{server_url}/user/{username}/outbox?page=true&limit={limit}");

    if page {
        let filters = TimelineFilters {
            view: Some(TimelineView::Global),
            hashtags: vec![],
            username: Some(username.clone()),
            conversation: None,
            excluded_words: vec![],
            direct: false,
        };

        Ok(Json(
            retrieve::activities(
                &conn,
                limit.into(),
                query.min,
                query.max,
                profile,
                filters,
                Some(base_url),
            )
            .await,
        ))
    } else if let Ok(profile) = get_actor_by_username(&conn, username).await {
        Ok(Json(
            retrieve::outbox_collection(&conn, profile, Some(base_url)).await,
        ))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// async fn axum_outbox_dispatch(
//     conn: &impl DbRunner,
//     pool: &deadpool_diesel::postgres::Pool,
//     profile: Actor,
//     activity: ApActivity,
//     raw: Value,
// ) -> Result<ActivityJson<ApActivity>, StatusCode> {
//     activity.outbox(conn, pool.clone(), profile, raw).await
// }

pub async fn axum_outbox_post(
    State(state): State<AppState>,
    Path(_username): Path<String>,
    signed: AxumSigned,
    Json(raw): Json<Value>,
) -> Result<ActivityJson<ApActivity>, StatusCode> {
    let profile = signed.profile().ok_or(StatusCode::UNAUTHORIZED)?;

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Ok(object) = serde_json::from_value::<ActivityPub>(raw.clone()) {
        match object {
            ActivityPub::Activity(activity) => {
                activity
                    .outbox(&conn, state.db_pool, profile, raw.clone())
                    .await
            }
            ActivityPub::Object(object) => object.outbox(&conn, state.db_pool, profile, raw).await,
            _ => {
                create_unprocessable(&conn, raw.into()).await;
                Err(StatusCode::NOT_IMPLEMENTED)
            }
        }
    } else {
        create_unprocessable(&conn, raw.into()).await;
        Err(StatusCode::UNPROCESSABLE_ENTITY)
    }
}
