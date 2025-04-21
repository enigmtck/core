use std::fmt::Display;

use super::Inbox;
use rocket::data::{self, Data, FromData, ToByteUnit};
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::Request;
use rocket::serde::json::Json;
use rocket::{get, post};
use serde_json::Value;

use crate::db::Db;
use crate::fairings::access_control::Permitted;
use crate::fairings::events::EventChannels;
use crate::fairings::signatures::Signed;
use crate::models::activities::get_announcers;
use crate::models::activities::{TimelineFilters, TimelineView};
use crate::models::leaders::get_leaders_by_actor_id;
use crate::models::unprocessable::create_unprocessable;
use crate::retriever;
use crate::signing::{get_hash, verify, VerificationType};
use crate::SERVER_URL;
use jdt_activity_pub::{
    verify_jsonld_signature, ActivityPub, ApActivity, ApActor, ApCollection, ApObject,
};
use std::fmt;
use urlencoding::encode;

use super::{retrieve, ActivityJson};

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

pub struct HashedJson {
    pub hash: String,
    pub json: Value,
}

#[rocket::async_trait]
impl<'r> FromData<'r> for HashedJson {
    type Error = anyhow::Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        let limit = req.limits().get("json").unwrap_or(1.mebibytes());

        let bytes = match data.open(limit).into_bytes().await {
            Ok(bytes) if bytes.is_complete() => bytes.into_inner(),
            Ok(_) => {
                return Outcome::Error((
                    Status::PayloadTooLarge,
                    anyhow::anyhow!("JSON POST too large"),
                ))
            }
            Err(e) => {
                return Outcome::Error((
                    Status::InternalServerError,
                    anyhow::anyhow!("IO error: {}", e),
                ))
            }
        };

        let hash = get_hash(bytes.clone());
        let json = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(e) => {
                return Outcome::Error((Status::BadRequest, anyhow::anyhow!("Invalid JSON: {}", e)))
            }
        };

        Outcome::Success(HashedJson { hash, json })
    }
}

#[post("/user/<_>/inbox", data = "<hashed>")]
pub async fn inbox_post(
    hashed: HashedJson,
    permitted: Permitted,
    signed: Signed,
    conn: Db,
    channels: EventChannels,
) -> Result<Status, Status> {
    shared_inbox_post(hashed, permitted, signed, conn, channels).await
}

#[post("/inbox", data = "<hashed>")]
pub async fn shared_inbox_post(
    hashed: HashedJson,
    permitted: Permitted,
    signed: Signed,
    conn: Db,
    _channels: EventChannels,
) -> Result<Status, Status> {
    if !permitted.is_permitted() {
        log::debug!("Request was explicitly forbidden");
        return Err(Status::Forbidden);
    }

    let raw = hashed.json;

    if let Some(signed_digest) = signed.digest() {
        let signed_digest = if signed_digest[..8].eq_ignore_ascii_case("sha-256=") {
            signed_digest[8..].to_string()
        } else {
            signed_digest
        };

        if hashed.hash != signed_digest {
            log::warn!("Computed body hash does not match signed hash");
            log::debug!("Computed JSON message digest: {}", hashed.hash);
            log::debug!("Signed JSON message digest: {signed_digest}");
            return Err(Status::Unauthorized);
        }
    }

    let activity: ApActivity = match raw.clone().try_into() {
        Ok(activity) => activity,
        Err(e) => {
            create_unprocessable(&conn, (raw.clone(), Some(format!("{e:#?}"))).into()).await;
            return Err(Status::UnprocessableEntity);
        }
    };

    log::info!("{}", activity);
    if activity.is_delete() && signed.deferred().is_some() {
        return Ok(Status::Accepted);
    }

    let mut signer = signed.actor();

    let is_authorized = if let Some(deferred) = signed.deferred() {
        let actor =
            match retriever::get_actor(&conn, activity.actor().to_string(), None, true).await {
                Ok(actor) => {
                    if let Some(id) = actor.id.clone() {
                        log::debug!("Deferred Actor retrieved: {id}");
                    }
                    Some(actor)
                }
                Err(e) => {
                    log::warn!("Failed to retrieve deferred actor: {e}");
                    None
                }
            };

        signer = actor;

        matches!(
            verify(&conn, deferred).await,
            Ok(VerificationType::Remote(_))
        )
    } else {
        signed.any()
    };

    // Skipping for now because verify_jsonld_signature generates too many requests for Context using
    // ReqwestLoader and they get blocked by Cloudflare
    // if activity.is_signed() {
    //     if let Some(actor) = signer {
    //         let public_key_pem = actor.public_key.public_key_pem;

    //         match verify_jsonld_signature(raw.clone(), public_key_pem).await {
    //             Ok(verified) => log::debug!("RsaSignature2017 Verification: {verified:#?}"),
    //             Err(e) => log::warn!("RsaSignature2017 Verification Error: {e}"),
    //         }
    //     }
    // }

    if is_authorized {
        activity.inbox(conn, raw).await
    } else {
        log::warn!("Request was not authorized");
        Err(Status::Unauthorized)
    }
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
                    excluded_words: vec![],
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
                    excluded_words: vec![],
                    direct: false,
                },
                InboxView::Local => TimelineFilters {
                    view: Some(view.into()),
                    hashtags,
                    username: None,
                    conversation: None,
                    excluded_words: vec![],
                    direct: false,
                },
                InboxView::Direct => TimelineFilters {
                    view: Some(view.into()),
                    hashtags,
                    username: None,
                    conversation: None,
                    excluded_words: vec![],
                    direct: true,
                },
            }
        } else {
            TimelineFilters {
                view: Some(TimelineView::Global),
                hashtags,
                username: None,
                conversation: None,
                excluded_words: vec![],
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

#[get("/api/announcers?<limit>&<min>&<max>&<target>")]
pub async fn announcers_get(
    permitted: Permitted,
    signed: Signed,
    conn: Db,
    target: String,
    min: Option<i64>,
    max: Option<i64>,
    limit: Option<u8>,
) -> Result<ActivityJson<ApObject>, Status> {
    if !permitted.is_permitted() {
        return Err(Status::Forbidden);
    }

    if !signed.local() {
        return Err(Status::Unauthorized);
    }

    let server_url = &*SERVER_URL;
    let limit = limit.unwrap_or(50);
    let base_url = format!("{server_url}/api/announcers?limit={limit}&target={target}");

    let decoded = urlencoding::decode(&target).map_err(|e| {
        log::error!("Failed to decode target: {e}");
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
        excluded_words: vec![],
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
