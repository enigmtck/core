use std::fmt::Display;

use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, post};
use serde_json::Value;

use crate::activity_pub::{
    retriever, ActivityPub, ApActivity, ApActor, ApCollection, ApObject, Inbox,
};
use crate::db::Db;
use crate::fairings::access_control::Permitted;
use crate::fairings::events::EventChannels;
use crate::fairings::signatures::Signed;
use crate::models::activities::{TimelineFilters, TimelineView};
use crate::models::leaders::get_leaders_by_actor_id;
use crate::models::pg::activities::get_announcers;
use crate::models::unprocessable::create_unprocessable;
use crate::signing::{verify, VerificationType};
use crate::SERVER_URL;
use std::fmt;
use urlencoding::encode;

use super::{retrieve, ActivityJson};

#[derive(FromFormField, Eq, PartialEq, Debug, Clone)]
pub enum InboxView {
    Home,
    Local,
    Global,
    Direct,
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

pub fn convert_hashtags_to_query_string(hashtags: &[String]) -> String {
    hashtags
        .iter()
        .map(|tag| format!("&hashtags[]={}", encode(tag)))
        .collect::<Vec<String>>()
        .join("")
}

pub fn add_hash_to_tags(hashtags: &[String]) -> Vec<String> {
    hashtags
        .iter()
        .map(|tag| format!("#{tag}"))
        .collect::<Vec<String>>()
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
            convert_hashtags_to_query_string(&hashtags)
        } else {
            String::new()
        }
    };

    let base_url =
        format!("{server_url}/inbox?page=true&limit={limit}{view_query}{hashtags_query}");

    let hashtags = if let Some(hashtags) = hashtags.clone() {
        add_hash_to_tags(&hashtags)
    } else {
        vec![]
    };

    let filters = {
        if let Some(view) = view {
            match view {
                InboxView::Global => TimelineFilters {
                    view: Some(view.into()),
                    hashtags,
                    username: None,
                    conversation: None,
                    direct: false,
                },
                InboxView::Home => TimelineFilters {
                    view: if let Some(profile) = profile.clone() {
                        Some(TimelineView::Home(
                            get_leaders_by_actor_id(&conn, profile.id, None)
                                .await
                                .iter()
                                .filter_map(|leader| leader.1.clone()?.as_followers.clone())
                                .collect(),
                        ))
                    } else {
                        Some(TimelineView::Global)
                    },
                    hashtags,
                    username: None,
                    conversation: None,
                    direct: false,
                },
                InboxView::Local => TimelineFilters {
                    view: Some(view.into()),
                    hashtags,
                    username: None,
                    conversation: None,
                    direct: false,
                },
                InboxView::Direct => TimelineFilters {
                    view: Some(view.into()),
                    hashtags,
                    username: None,
                    conversation: None,
                    direct: true,
                },
            }
        } else {
            TimelineFilters {
                view: Some(TimelineView::Global),
                hashtags,
                username: None,
                conversation: None,
                direct: false,
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
    log::debug!("Posting to unsafe inbox\n{raw:#?}");
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
    if !permitted.is_permitted() {
        log::debug!("Request was explicitly forbidden");
        return Err(Status::Forbidden);
    }

    let raw = raw.into_inner();

    let activity = match serde_json::from_value::<ApActivity>(raw.clone()) {
        Ok(activity) => activity,
        Err(_) => {
            create_unprocessable(&conn, raw.into()).await;
            return Err(Status::UnprocessableEntity);
        }
    };

    let is_authorized = if let Some(deferred) = signed.deferred() {
        let actor = retriever::get_actor(&conn, activity.actor().to_string(), None, true).await;

        log::debug!("Deferred Actor retrieved\n{actor:#?}");

        matches!(
            verify(&conn, deferred).await,
            Ok(VerificationType::Remote(_))
        )
    } else {
        signed.any()
    };

    if is_authorized {
        activity.inbox(conn, channels, raw).await
    } else {
        log::debug!("Request was not authorized");
        Err(Status::Unauthorized)
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

    let decoded = urlencoding::decode(&target).map_err(|e| {
        log::error!("Failed to decode target: {e:#?}");
        Status::UnprocessableEntity
    })?;

    let actors = get_announcers(&conn, min, max, Some(limit), decoded.to_string())
        .await
        .iter()
        .map(ApActor::from)
        .map(ActivityPub::from)
        .collect();

    Ok(ActivityJson(Json(ApObject::Collection(
        ApCollection::from((actors, Some(base_url))),
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
    let decoded = urlencoding::decode(&id).map_err(|e| {
        log::error!("Failed to decode id: {e:#?}");
        Status::UnprocessableEntity
    })?;

    let limit = limit.unwrap_or(20);
    let server_url = &*SERVER_URL;
    let base_url = format!("{server_url}/api/conversation?id={id}&limit={limit}");

    let filters = TimelineFilters {
        view: None,
        hashtags: vec![],
        username: None,
        conversation: Some(decoded.to_string()),
        direct: false,
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
