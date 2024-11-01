use std::fmt::Display;

use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, post};
use serde_json::Value;

use crate::activity_pub::{ActivityPub, ApActivity, ApActor, ApCollectionPage, ApObject, Inbox};
use crate::db::Db;
use crate::fairings::access_control::Permitted;
use crate::fairings::events::EventChannels;
use crate::fairings::signatures::Signed;
use crate::models::activities::{TimelineFilters, TimelineView};
use crate::models::leaders::get_leaders_by_actor_id;
use crate::models::pg::activities::get_announcers;
use crate::models::unprocessable::create_unprocessable;
use crate::SERVER_URL;
use std::fmt;
//use crate::models::remote_activities::create_remote_activity;

use super::{retrieve, ActivityJson};

#[derive(FromFormField, Eq, PartialEq, Debug, Clone)]
pub enum InboxView {
    Home,
    Local,
    Global,
}

impl Display for InboxView {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:#?}", self)
    }
}

#[allow(clippy::too_many_arguments)]
#[get("/user/<_username>/inbox?<min>&<max>&<limit>&<view>&<hashtags>")]
pub async fn inbox_get(
    signed: Signed,
    conn: Db,
    _username: String,
    min: Option<i64>,
    max: Option<i64>,
    limit: u8,
    view: InboxView,
    hashtags: Option<Vec<String>>,
) -> Result<ActivityJson<ApObject>, Status> {
    shared_inbox_get(signed, conn, min, max, limit, Some(view), hashtags).await
}

#[post("/user/<_>/inbox", data = "<activity>")]
pub async fn inbox_post(
    permitted: Permitted,
    signed: Signed,
    conn: Db,
    channels: EventChannels,
    activity: Json<Value>,
) -> Result<Status, Status> {
    shared_inbox_post(permitted, signed, conn, channels, activity).await
}

#[get("/inbox?<min>&<max>&<limit>&<hashtags>&<view>")]
pub async fn shared_inbox_get(
    signed: Signed,
    conn: Db,
    min: Option<i64>,
    max: Option<i64>,
    limit: u8,
    view: Option<InboxView>,
    hashtags: Option<Vec<String>>,
) -> Result<ActivityJson<ApObject>, Status> {
    let profile = signed.profile();
    let server_url = &*SERVER_URL;

    let view_query = {
        if let Some(view) = view.clone() {
            format!("&view={}", view)
        } else {
            String::new()
        }
    };

    let hashtags_query = {
        if let Some(hashtags) = hashtags.clone() {
            hashtags
                .iter()
                .map(|h| format!("hashtag[]={}", h))
                .collect::<Vec<String>>()
                .join("&")
        } else {
            String::new()
        }
    };

    let base_url =
        format!("{server_url}/inbox?page=true&limit={limit}{view_query}{hashtags_query}");

    let filters = {
        if let Some(view) = view {
            match view {
                InboxView::Global => TimelineFilters {
                    view: Some(view.into()),
                    hashtags: hashtags.unwrap_or_default(),
                    username: None,
                    conversation: None,
                },
                InboxView::Home => TimelineFilters {
                    view: if let Some(profile) = profile.clone() {
                        Some(TimelineView::Home(
                            get_leaders_by_actor_id(&conn, profile.id)
                                .await
                                .iter()
                                .filter_map(|leader| leader.1.clone()?.as_followers.clone())
                                .collect(),
                        ))
                    } else {
                        Some(TimelineView::Global)
                    },
                    hashtags: hashtags.unwrap_or_default(),
                    username: None,
                    conversation: None,
                },
                InboxView::Local => TimelineFilters {
                    view: Some(view.into()),
                    hashtags: hashtags.unwrap_or_default(),
                    username: None,
                    conversation: None,
                },
            }
        } else {
            TimelineFilters {
                view: Some(TimelineView::Global),
                hashtags: hashtags.unwrap_or_default(),
                username: None,
                conversation: None,
            }
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
}

#[post("/unsafe-inbox", data = "<raw>")]
pub async fn unsafe_inbox_post(
    conn: Db,
    channels: EventChannels,
    raw: Json<Value>,
) -> Result<Status, Status> {
    log::debug!("POSTING TO UNSAFE INBOX\n{raw:#?}");
    let raw = raw.into_inner();

    if let Ok(activity) = serde_json::from_value::<ApActivity>(raw.clone()) {
        activity.inbox(conn, channels, raw).await
    } else {
        create_unprocessable(&conn, raw.into()).await;
        Err(Status::UnprocessableEntity)
    }
}

#[post("/inbox", data = "<raw>")]
pub async fn shared_inbox_post(
    permitted: Permitted,
    signed: Signed,
    conn: Db,
    channels: EventChannels,
    raw: Json<Value>,
) -> Result<Status, Status> {
    if permitted.is_permitted() {
        // let raw =
        //     serde_json::from_str::<Value>(&activity).map_err(|_| Status::UnprocessableEntity)?;
        //log::debug!("POSTING TO INBOX\n{raw:#?}");
        let raw = raw.into_inner();

        if let Ok(activity) = serde_json::from_value::<ApActivity>(raw.clone()) {
            //log::debug!("POSTING TO INBOX\n{activity:#?}");

            if signed.any() {
                activity.inbox(conn, channels, raw).await
            } else {
                log::debug!("REQUEST WAS UNSIGNED OR MALFORMED");
                Err(Status::Unauthorized)
            }
        } else {
            create_unprocessable(&conn, raw.into()).await;
            Err(Status::UnprocessableEntity)
        }
    } else {
        log::debug!("REQUEST WAS EXPLICITLY PROHIBITED");
        Err(Status::Forbidden)
    }
}

#[get("/api/announcers?<limit>&<min>&<max>&<target>")]
pub async fn announcers_get(
    _permitted: Permitted,
    _signed: Signed,
    conn: Db,
    target: String,
    min: Option<i64>,
    max: Option<i64>,
    limit: Option<u8>,
) -> Result<ActivityJson<ApObject>, Status> {
    //if permitted.is_permitted() {
    //if signed.local() {
    let server_url = &*SERVER_URL;
    let limit = limit.unwrap_or(50);
    let base_url = format!("{server_url}/api/announcers?limit={limit}&target={target}");

    let decoded = urlencoding::decode(&target).map_err(|_| Status::UnprocessableEntity)?;

    let actors = get_announcers(&conn, min, max, Some(limit), decoded.to_string())
        .await
        .iter()
        .map(ApActor::from)
        .map(ActivityPub::from)
        .collect();

    Ok(ActivityJson(Json(ApObject::CollectionPage(
        ApCollectionPage::from((actors, Some(base_url))),
    ))))
    // } else {
    //     Err(Status::Unauthorized)
    // }
    // } else {
    //     Err(Status::Forbidden)
    // }
}

#[get("/api/conversation?<id>&<min>&<max>&<limit>")]
pub async fn conversation_get(
    signed: Signed,
    conn: Db,
    id: String,
    limit: Option<u8>,
    min: Option<i64>,
    max: Option<i64>,
) -> Result<ActivityJson<ApObject>, Status> {
    let decoded = urlencoding::decode(&id).map_err(|_| Status::UnprocessableEntity)?;
    let limit = limit.unwrap_or(20);
    let server_url = &*SERVER_URL;
    let base_url = format!("{server_url}/api/conversation?id={id}&limit={limit}");

    //log::debug!("RETRIEVING CONVERSATION: {decoded}");

    let filters = TimelineFilters {
        view: Some(TimelineView::Global),
        hashtags: vec![],
        username: None,
        conversation: Some(decoded.to_string()),
    };

    Ok(ActivityJson(Json(
        retrieve::activities(
            &conn,
            limit.into(),
            min,
            max,
            signed.profile(),
            filters,
            Some(base_url),
        )
        .await,
    )))
}
