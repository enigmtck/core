use super::{ActivityJson, Inbox};
use crate::{
    blocklist::Permitted,
    models::{
        activities::{get_announcers, TimelineFilters, TimelineView},
        follows::get_leaders_by_follower_actor_id,
        unprocessable::create_unprocessable,
    },
    retriever::{self, get_actor},
    server::{extractors::AxumSigned, AppState},
    signing::{get_hash, verify, VerificationError},
};
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
};
use axum_extra::extract::Query;
use jdt_activity_pub::{ActivityPub, ApActivity, ApActor, ApCollection, ApObject};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::fmt;
use std::fmt::Display;
use urlencoding::encode;

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

pub fn sanitize_json_fields(mut value: Value) -> Value {
    if let Value::Object(ref mut obj) = value {
        // Handle top level: remove "attributedTo" if both exist and are identical
        sanitize_level(obj, "actor", "attributedTo");

        // Handle conversation/context overlap at top level
        sanitize_level(obj, "conversation", "context");

        // Handle one level deeper in "object" field
        if let Some(Value::Object(ref mut object_obj)) = obj.get_mut("object") {
            // In object level: remove "actor" if both exist and are identical
            sanitize_level(object_obj, "attributedTo", "actor");

            // Handle conversation/context overlap in object level
            sanitize_level(object_obj, "conversation", "context");
        }
    }
    value
}

fn sanitize_level(obj: &mut Map<String, Value>, keep_field: &str, remove_field: &str) {
    if let (Some(keep_val), Some(remove_val)) = (obj.get(keep_field), obj.get(remove_field)) {
        // If both are identical, remove the unwanted field
        if keep_val == remove_val {
            obj.remove(remove_field);
        }
        // If one is null, consolidate to the desired survivor
        else if keep_val.is_null() && !remove_val.is_null() {
            obj.insert(keep_field.to_string(), remove_val.clone());
            obj.remove(remove_field);
        }
        // If remove_val is null (regardless of keep_val), remove the unwanted field
        else if remove_val.is_null() {
            obj.remove(remove_field);
        }
        // If they're different and neither is null, log warning and remove unwanted field
        else {
            log::warn!(
                "Mismatch between {keep_field} and {remove_field}: {keep_val} vs {remove_val}"
            );
            obj.remove(remove_field);
        }
    }
    // If only one exists or neither exists - no action needed
}

#[derive(Eq, PartialEq, Debug, Clone, Deserialize)]
pub enum InboxView {
    #[serde(alias = "home")]
    Home,
    #[serde(alias = "local")]
    Local,
    #[serde(alias = "global")]
    Global,
    #[serde(alias = "direct")]
    Direct,
}

impl Display for InboxView {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:#?}")
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

#[derive(Deserialize, Debug)]
pub struct InboxQuery {
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub limit: Option<u8>,
    pub view: Option<InboxView>,
    #[serde(rename = "hashtags[]")]
    pub hashtags: Option<Vec<String>>,
}

pub struct AxumHashedJson {
    pub hash: String,
    pub json: Value,
}

#[axum::debug_handler]
pub async fn axum_shared_inbox_get(
    State(app_state): State<AppState>,
    Query(query): Query<InboxQuery>,
    _username: Option<Path<String>>,
    signed: AxumSigned,
) -> Result<ActivityJson<ApObject>, StatusCode> {
    log::debug!("{query:?}");

    let conn = match app_state.db_pool.get().await {
        Ok(conn) => conn,
        Err(e) => {
            log::error!("Failed to get DB connection from pool: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let profile = signed.profile();
    let server_url = format!("https://{}", *crate::SERVER_NAME);

    let view_query = {
        if let Some(view) = query.view.clone() {
            format!("&view={view}")
        } else {
            String::new()
        }
    };

    let hashtags_query = {
        if let Some(hashtags) = query.hashtags.clone() {
            convert_hashtags_to_query_string(&hashtags)
        } else {
            String::new()
        }
    };

    let base_url = format!(
        "{server_url}/inbox?page=true&limit={}{view_query}{hashtags_query}",
        query.limit.unwrap_or(20)
    );

    let hashtags = if let Some(hashtags) = query.hashtags.clone() {
        add_hash_to_tags(&hashtags)
    } else {
        vec![]
    };

    let filters = if let Some(view) = query.view {
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
                    match get_leaders_by_follower_actor_id(&conn, profile.id, None).await {
                        Ok(leaders) => Some(TimelineView::Home(
                            leaders
                                .iter()
                                .filter_map(|leader| leader.1.clone()?.as_followers)
                                .collect(),
                        )),
                        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
                    }
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
    };

    let result = retriever::activities(
        &conn,
        query.limit.unwrap_or(20).into(),
        query.min,
        query.max,
        profile,
        filters,
        Some(base_url),
    )
    .await;

    Ok(ActivityJson(result))
}

pub async fn axum_shared_inbox_post(
    State(state): State<AppState>,
    _username: Option<Path<String>>,
    signed: AxumSigned,
    permitted: Permitted,
    bytes: Bytes,
) -> Result<StatusCode, StatusCode> {
    // Reject connection immediately if from a prohibited server
    if !permitted.is_permitted() {
        return Err(StatusCode::FORBIDDEN);
    }

    // Reject connection if the hash cannot be computed or if it doesn't match
    let hash = get_hash(bytes.to_vec());
    let json: Value = match serde_json::from_slice(&bytes) {
        Ok(j) => j,
        Err(e) => {
            log::error!("Failed to parse JSON from request body: {e}");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let hashed = AxumHashedJson { hash, json };

    if let Some(signed_digest) = signed.digest() {
        let signed_digest = signed_digest.strip_prefix("sha-256=").unwrap_or(
            signed_digest
                .strip_prefix("SHA-256=")
                .unwrap_or(&signed_digest),
        );

        if hashed.hash != signed_digest {
            log::debug!("Failed to verify hash: {}, {signed_digest}", hashed.hash);
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // Clean up the JSON from servers like Akkoma that send spurious fields
    let raw = sanitize_json_fields(hashed.json);

    // Wait until absolutely necessary to reserve a database connection
    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Reject message if an ApActivity can not be built from it; log to the unprocessable table
    let activity: ApActivity = match raw.clone().try_into() {
        Ok(activity) => activity,
        Err(e) => {
            create_unprocessable(&conn, (raw, Some(format!("{e:#?}"))).into()).await;
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    // If this is a Delete and we don't have the Actor, just Accept it and do nothing
    if activity.is_delete() && signed.deferred().is_some() {
        log::debug!("Accepting Delete activity for non-existent Actor");
        return Ok(StatusCode::ACCEPTED);
    }

    // Handle the Deferred Actor here
    let is_authorized = if let Some(deferred) = signed.deferred() {
        match verify(&conn, deferred).await {
            Ok(_) => true,
            Err(VerificationError::ActorNotFound(_)) => {
                let actor = activity.actor().to_string();
                log::debug!("Attempting to retrieve {actor}");
                get_actor(&conn, actor, None, true).await.is_ok()
            }
            _ => false,
        }
    } else {
        signed.any()
    };

    if is_authorized {
        activity.inbox(&conn, state.db_pool.clone(), raw).await
    } else {
        log::debug!("Request signature verification failed");
        Err(StatusCode::UNAUTHORIZED)
    }
}

#[derive(Deserialize, Debug)]
pub struct AnnouncersQuery {
    pub limit: Option<u8>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub target: String,
}

pub async fn axum_announcers_get(
    State(state): State<AppState>,
    signed: AxumSigned,
    permitted: Permitted,
    Query(query): Query<AnnouncersQuery>,
) -> Result<ActivityJson<ApObject>, StatusCode> {
    if !permitted.is_permitted() {
        return Err(StatusCode::FORBIDDEN);
    }

    if !signed.local() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let server_url = format!("https://{}", *crate::SERVER_NAME);
    let limit = query.limit.unwrap_or(50);
    let base_url = format!(
        "{server_url}/api/announcers?limit={limit}&target={}",
        query.target
    );

    let decoded =
        urlencoding::decode(&query.target).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    let actors = get_announcers(
        &conn,
        query.min,
        query.max,
        Some(limit),
        decoded.to_string(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(ApActor::from)
    .map(ActivityPub::from)
    .collect();

    Ok(ActivityJson(ApObject::Collection(ApCollection::from((
        actors,
        Some(base_url),
    )))))
}

#[derive(Deserialize, Debug)]
pub struct ConversationQuery {
    pub id: String,
    pub limit: Option<u8>,
    pub min: Option<i64>,
    pub max: Option<i64>,
}

pub async fn axum_conversation_get(
    State(state): State<AppState>,
    signed: AxumSigned,
    Query(query): Query<ConversationQuery>,
) -> Result<axum::Json<ApObject>, StatusCode> {
    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let decoded = urlencoding::decode(&query.id).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    let limit = query.limit.unwrap_or(20);
    let server_url = format!("https://{}", *crate::SERVER_NAME);
    let base_url = format!(
        "{server_url}/api/conversation?id={}&limit={limit}",
        query.id
    );

    let filters = TimelineFilters {
        view: None,
        hashtags: vec![],
        username: None,
        conversation: Some(decoded.to_string()),
        excluded_words: vec![],
        direct: false,
    };

    Ok(axum::Json(
        retriever::activities(
            &conn,
            limit.into(),
            query.min,
            query.max,
            signed.profile(),
            filters,
            Some(base_url),
        )
        .await,
    ))
}
