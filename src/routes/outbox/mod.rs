use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::fairings::signatures::Signed;
use crate::models::activities::{TimelineFilters, TimelineView};
use crate::models::actors::get_actor_by_username;
use crate::models::unprocessable::create_unprocessable;
use crate::routes::Outbox;
use crate::SERVER_URL;
use jdt_activity_pub::{ActivityPub, ApActivity, ApObject};
use rocket::{get, http::Status, post, serde::json::Json, serde::json::Value};

use super::{retrieve, ActivityJson};

// Activities
pub mod accept;
pub mod add;
pub mod announce;
pub mod block;
pub mod create;
pub mod delete;
pub mod follow;
pub mod like;
pub mod undo;
pub mod update;

// Objects
pub mod actor;
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

#[get("/user/<username>/outbox?<limit>&<min>&<max>&<page>")]
pub async fn outbox_get(
    signed: Signed,
    conn: Db,
    username: String,
    min: Option<i64>,
    max: Option<i64>,
    limit: Option<u8>,
    page: Option<bool>,
) -> Result<ActivityJson<ApObject>, Status> {
    let profile = signed.profile();
    let server_url = &*SERVER_URL;
    let limit = limit.unwrap_or(10);
    let page = page.unwrap_or_default();

    let base_url = format!("{server_url}/user/{username}/outbox?page=true&limit={limit}");

    if page {
        let filters = {
            TimelineFilters {
                view: Some(TimelineView::Global),
                hashtags: vec![],
                username: Some(username.clone()),
                conversation: None,
                excluded_words: vec![],
                direct: false,
            }
        };

        Ok(ActivityJson(Json(
            retrieve::activities(
                &conn,
                limit.into(),
                min,
                max,
                profile,
                filters,
                Some(base_url),
            )
            .await,
        )))
    } else if let Some(profile) = get_actor_by_username(&conn, username.clone()).await {
        Ok(ActivityJson(Json(
            retrieve::outbox_collection(&conn, profile, Some(base_url)).await,
        )))
    } else {
        Err(Status::NotFound)
    }
}

#[post("/user/<_username>/outbox", data = "<raw>")]
pub async fn outbox_post(
    signed: Signed,
    conn: Db,
    _events: EventChannels,
    _username: String,
    raw: Json<Value>,
) -> Result<ActivityJson<ApActivity>, Status> {
    let actor = signed.profile().ok_or(Status::Unauthorized)?;

    log::debug!("Posting to Outbox\n{raw:#?}");
    let raw = raw.into_inner();

    if let Ok(object) = serde_json::from_value::<ActivityPub>(raw.clone()) {
        match object {
            ActivityPub::Activity(activity) => activity.outbox(conn, actor, raw).await,
            ActivityPub::Object(object) => object.outbox(conn, actor, raw).await,
            _ => {
                let unprocessable = create_unprocessable(&conn, raw.into()).await;
                log::error!(
                    "Unprocessable: {}",
                    unprocessable
                        .map(|x| x.id.to_string())
                        .unwrap_or("Failed".to_string())
                );
                Err(Status::NotImplemented)
            }
        }
    } else {
        let unprocessable = create_unprocessable(&conn, raw.into()).await;
        log::error!(
            "Unprocessable: {}",
            unprocessable
                .map(|x| x.id.to_string())
                .unwrap_or("Failed".to_string())
        );
        Err(Status::UnprocessableEntity)
    }
}
