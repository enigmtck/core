use crate::activity_pub::{ActivityPub, ApObject, Outbox};
use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::fairings::signatures::Signed;
use crate::models::activities::{TimelineFilters, TimelineView};
use crate::models::profiles::get_profile_by_username;
//use crate::models::timeline::{TimelineFilters, TimelineView};
use crate::SERVER_URL;
use rocket::{get, http::Status, post, serde::json::Error, serde::json::Json};

use super::{retrieve, ActivityJson};

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
                view: TimelineView::Global,
                hashtags: vec![],
                username: Some(username.clone()),
                conversation: None,
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
    } else if let Some(profile) = get_profile_by_username(Some(&conn), username.clone()).await {
        Ok(ActivityJson(Json(
            retrieve::outbox_collection(&conn, profile, Some(base_url)).await,
        )))
    } else {
        Err(Status::NotFound)
    }
}

#[post("/user/<username>/outbox", data = "<object>")]
pub async fn outbox_post(
    signed: Signed,
    conn: Db,
    events: EventChannels,
    username: String,
    object: Result<Json<ActivityPub>, Error<'_>>,
) -> Result<String, Status> {
    log::debug!("POSTING TO OUTBOX\n{object:#?}");

    if signed.local() {
        let profile = get_profile_by_username((&conn).into(), username)
            .await
            .ok_or(Status::new(521))?;

        let object = object.map_err(|_| Status::new(522))?;

        match object {
            Json(ActivityPub::Activity(activity)) => activity.outbox(conn, events, profile).await,
            Json(ActivityPub::Object(object)) => object.outbox(conn, events, profile).await,
            _ => Err(Status::new(523)),
        }
    } else {
        Err(Status::Unauthorized)
    }
}
