use super::ActivityJson;
use crate::server::retriever;
use crate::server::routes::Outbox;
use crate::{
    models::{
        activities::{TimelineFilters, TimelineView},
        actors::get_actor_by_username,
        unprocessable::create_unprocessable,
    },
    server::{extractors::AxumSigned, AppState},
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use jdt_activity_pub::{ActivityPub, ApActivity, ApObject};
use serde::Deserialize;
use serde_json::Value;

// Activities
pub mod accept;
pub mod add;
pub mod announce;
pub mod ap_move;
pub mod block;
pub mod create;
pub mod delete;
pub mod follow;
pub mod like;
pub mod remove;
pub mod undo;
pub mod update;

// Objects
pub mod actor;
pub mod article;
pub mod collection;
pub mod complex;
pub mod identifier;
pub mod instrument;
pub mod note;
pub mod plain;
pub mod question;
pub mod session;
pub mod tombstone;
pub mod uncategorized;

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

    //let base_url = format!("{server_url}/user/{username}/outbox?page=true&limit={limit}");
    let base_url = format!("{server_url}/user/{username}/outbox");

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
            retriever::activities(
                &conn,
                limit.into(),
                query.min,
                query.max,
                profile,
                filters,
                format!("{base_url}?page=true&limit={limit}"),
            )
            .await,
        ))
    } else if let Ok(profile) = get_actor_by_username(&conn, username).await {
        Ok(Json(
            retriever::outbox_collection(&conn, profile, limit).await,
        ))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

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
                    .outbox(&conn, state.clone(), profile, raw.clone())
                    .await
            }
            ActivityPub::Object(object) => object.outbox(&conn, state, profile, raw).await,
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
