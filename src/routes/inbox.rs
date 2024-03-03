use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, post};
use serde_json::Value;

use crate::activity_pub::{ApActivity, ApObject, Inbox};
use crate::db::Db;
use crate::fairings::faktory::FaktoryConnection;
use crate::fairings::signatures::Signed;
use crate::inbox;
use crate::models::leaders::get_leaders_by_profile_id;
use crate::models::{
    profiles::get_profile_by_username, timeline::TimelineFilters, timeline::TimelineView,
};
//use crate::models::remote_activities::create_remote_activity;

use super::ActivityJson;

#[derive(FromFormField, Eq, PartialEq)]
pub enum InboxView {
    Home,
    Local,
    Global,
}

#[get("/user/<username>/inbox?<offset>&<limit>&<view>")]
pub async fn inbox_get(
    signed: Signed,
    conn: Db,
    username: String,
    offset: u16,
    limit: u8,
    view: InboxView,
) -> Result<ActivityJson<ApObject>, Status> {
    if signed.local() {
        let profile = get_profile_by_username((&conn).into(), username)
            .await
            .ok_or(Status::new(520))?;

        let filters = TimelineFilters {
            view: {
                if view == InboxView::Home {
                    TimelineView::Home(
                        get_leaders_by_profile_id(&conn, profile.id)
                            .await
                            .iter()
                            .map(|leader| leader.0.leader_ap_id.clone())
                            .collect(),
                    )
                } else {
                    view.into()
                }
            },
            hashtags: vec![],
        };

        Ok(ActivityJson(Json(
            inbox::retrieve::inbox(&conn, limit.into(), offset.into(), profile, filters).await,
        )))
    } else {
        Err(Status::Unauthorized)
    }
}

#[get("/api/timeline?<offset>&<limit>")]
pub async fn timeline(conn: Db, offset: u16, limit: u8) -> Result<ActivityJson<ApObject>, Status> {
    Ok(ActivityJson(Json(
        inbox::retrieve::timeline(&conn, limit.into(), offset.into()).await,
    )))
}

#[post("/user/<_>/inbox", data = "<activity>")]
pub async fn inbox_post(
    signed: Signed,
    conn: Db,
    faktory: FaktoryConnection,
    activity: String,
) -> Result<Status, Status> {
    shared_inbox_post(signed, conn, faktory, activity).await
}

#[post("/inbox", data = "<activity>")]
pub async fn shared_inbox_post(
    signed: Signed,
    conn: Db,
    faktory: FaktoryConnection,
    activity: String,
) -> Result<Status, Status> {
    let raw = serde_json::from_str::<Value>(&activity).map_err(|_| Status::BadRequest)?;
    let activity = serde_json::from_str::<ApActivity>(&activity).map_err(|_| Status::BadRequest)?;

    log::debug!("POSTING TO INBOX\n{activity:#?}");

    if signed.any() {
        activity.inbox(conn, faktory, raw).await
    } else {
        log::debug!("REQUEST WAS UNSIGNED OR MALFORMED");
        Err(Status::NoContent)
    }
}

#[get("/api/user/<username>/conversation?<conversation>&<offset>&<limit>")]
pub async fn conversation_get(
    signed: Signed,
    conn: Db,
    faktory: FaktoryConnection,
    username: String,
    offset: u16,
    limit: u8,
    conversation: String,
) -> Result<ActivityJson<ApObject>, Status> {
    if signed.local() {
        if get_profile_by_username((&conn).into(), username)
            .await
            .is_some()
        {
            let decoded = urlencoding::decode(&conversation).map_err(|_| Status::new(524))?;

            inbox::retrieve::conversation(
                &conn,
                faktory,
                decoded.to_string(),
                limit.into(),
                offset.into(),
            )
            .await
            .map(|inbox| ActivityJson(Json(inbox)))
            .map_err(|_| Status::new(525))
        } else {
            Err(Status::new(526))
        }
    } else {
        Err(Status::Unauthorized)
    }
}

#[get("/conversation/<uuid>")]
pub async fn conversation_get_local(
    conn: Db,
    faktory: FaktoryConnection,
    uuid: String,
) -> Result<ActivityJson<ApObject>, Status> {
    let conversation = format!("{}/conversation/{}", *crate::SERVER_URL, uuid);

    inbox::retrieve::conversation(&conn, faktory, conversation.to_string(), 40, 0)
        .await
        .map(|conversation| ActivityJson(Json(conversation)))
        .map_err(|_| Status::new(525))
}
