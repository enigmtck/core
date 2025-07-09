use crate::{
    axum_server::{extractors::AxumSigned, AppState},
    fairings::access_control::Permitted,
    models::{
        activities::{get_announcers, TimelineFilters, TimelineView},
        follows::get_leaders_by_follower_actor_id,
        unprocessable::create_unprocessable,
    },
    retriever,
    routes::{
        inbox::{
            add_hash_to_tags, convert_hashtags_to_query_string, sanitize_json_fields, InboxView,
        },
        retrieve, ActivityJson, Inbox,
    },
    signing::{get_hash, verify, VerificationType},
};
use axum::{
    body::Bytes,
    extract::{Query, State},
    http::StatusCode,
};
use jdt_activity_pub::{ActivityPub, ApActivity, ApActor, ApCollection, ApObject};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct InboxQuery {
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub limit: Option<u8>,
    pub view: Option<InboxView>,
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
    signed: AxumSigned,
) -> Result<ActivityJson<ApObject>, StatusCode> {
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

// async fn handle_update<C: DbRunner>(
//     update: ApUpdate,
//     conn: &C,
//     raw: Value,
// ) -> Result<StatusCode, StatusCode> {
//     let activity: ApActivity = update.clone().into();

//     match update.clone().object {
//         MaybeReference::Actual(actual) => match actual {
//             ApObject::Actor(actor) => {
//                 let webfinger = actor.get_webfinger().await;
//                 let mut new_remote_actor = NewActor::try_from(actor.clone())
//                     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//                 new_remote_actor.ek_webfinger = webfinger;

//                 if actor.clone().id.unwrap_or_default() == update.actor.clone() {
//                     let actor = create_or_update_actor(conn, new_remote_actor)
//                         .await
//                         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//                     let mut activity = NewActivity::try_from((activity, Some(actor.into())))
//                         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//                     activity.raw = Some(raw);
//                     create_activity(conn, activity)
//                         .await
//                         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//                     Ok(StatusCode::ACCEPTED)
//                 } else {
//                     Err(StatusCode::UNAUTHORIZED)
//                 }
//             }
//             ApObject::Note(note) => {
//                 if note.clone().attributed_to == update.actor.clone() {
//                     let object = create_or_update_object(conn, note.into())
//                         .await
//                         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//                     let mut activity = NewActivity::try_from((activity, Some(object.into())))
//                         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//                     activity.raw = Some(raw);
//                     create_activity(conn, activity)
//                         .await
//                         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//                     Ok(StatusCode::ACCEPTED)
//                 } else {
//                     Err(StatusCode::UNAUTHORIZED)
//                 }
//             }
//             ApObject::Article(article) => {
//                 if article.clone().attributed_to == update.actor.clone() {
//                     let object = create_or_update_object(conn, article.into())
//                         .await
//                         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//                     let mut activity = NewActivity::try_from((activity, Some(object.into())))
//                         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//                     activity.raw = Some(raw);
//                     create_activity(conn, activity)
//                         .await
//                         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//                     Ok(StatusCode::ACCEPTED)
//                 } else {
//                     Err(StatusCode::UNAUTHORIZED)
//                 }
//             }
//             ApObject::Question(question) => {
//                 if question.clone().attributed_to == update.actor.clone() {
//                     let object = create_or_update_object(conn, question.into())
//                         .await
//                         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//                     let mut activity = NewActivity::try_from((activity, Some(object.into())))
//                         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//                     activity.raw = Some(raw);
//                     create_activity(conn, activity)
//                         .await
//                         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//                     Ok(StatusCode::ACCEPTED)
//                 } else {
//                     Err(StatusCode::UNAUTHORIZED)
//                 }
//             }
//             _ => Err(StatusCode::NOT_IMPLEMENTED),
//         },
//         _ => Err(StatusCode::UNPROCESSABLE_ENTITY),
//     }
// }

// async fn handle_undo<C: DbRunner>(
//     undo: Box<ApUndo>,
//     conn: &C,
//     db_pool: &deadpool_diesel::postgres::Pool,
//     raw: Value,
// ) -> Result<StatusCode, StatusCode> {
//     let target = match undo.object.clone() {
//         MaybeReference::Actual(actual) => actual,
//         _ => return Err(StatusCode::BAD_REQUEST),
//     };

//     let target_ap_id = target.as_id().ok_or(StatusCode::NOT_IMPLEMENTED)?;

//     let (_, target_activity, _, _) = get_activity_by_ap_id(conn, target_ap_id.clone())
//         .await
//         .map_err(|_| StatusCode::NOT_FOUND)?
//         .ok_or(StatusCode::NOT_FOUND)?;

//     let activity_target = (
//         ApActivity::Undo(undo.clone()),
//         Some(ActivityTarget::from(
//             target_activity.ok_or(StatusCode::NOT_FOUND)?,
//         )),
//     );

//     let mut activity =
//         NewActivity::try_from(activity_target).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//     activity.raw = Some(raw);
//     create_activity(conn, activity)
//         .await
//         .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//     match target {
//         ApActivity::Like(_) => {
//             revoke_activity_by_apid(conn, target_ap_id)
//                 .await
//                 .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//             Ok(StatusCode::ACCEPTED)
//         }
//         ApActivity::Follow(follow) => {
//             let follower_ap_id = follow.actor.to_string();
//             let leader_ap_id = follow
//                 .object
//                 .reference()
//                 .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
//             delete_follow(conn, follower_ap_id, leader_ap_id)
//                 .await
//                 .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//             revoke_activity_by_apid(conn, follow.id.ok_or(StatusCode::BAD_REQUEST)?)
//                 .await
//                 .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//             Ok(StatusCode::ACCEPTED)
//         }
//         ApActivity::Announce(_) => {
//             let db_pool = db_pool.clone();
//             tokio::spawn(async move {
//                 if let Ok(conn) = db_pool.get().await {
//                     if let Ok(activity) = revoke_activity_by_apid(&conn, target_ap_id).await {
//                         if let Ok(announcer) = get_actor_by_as_id(&conn, activity.actor).await {
//                             let inboxes =
//                                 runner::user::get_follower_inboxes(&conn, announcer.clone()).await;
//                             let message = ApActivity::Undo(undo);
//                             if let Err(e) =
//                                 runner::send_to_inboxes(&conn, inboxes, announcer, message).await
//                             {
//                                 log::error!("Failed to send undo announce: {e}");
//                             }
//                         }
//                     }
//                 }
//             });
//             Ok(StatusCode::ACCEPTED)
//         }
//         _ => Err(StatusCode::NOT_IMPLEMENTED),
//     }
// }

pub async fn axum_shared_inbox_post(
    State(state): State<AppState>,
    signed: AxumSigned,
    permitted: Permitted,
    bytes: Bytes,
) -> Result<StatusCode, StatusCode> {
    if !permitted.is_permitted() {
        return Err(StatusCode::FORBIDDEN);
    }

    let hash = get_hash(bytes.to_vec());
    let json: Value = match serde_json::from_slice(&bytes) {
        Ok(j) => j,
        Err(e) => {
            log::error!("Failed to parse JSON from request body: {e}");
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    let hashed = AxumHashedJson { hash, json };

    let raw = sanitize_json_fields(hashed.json);

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

    let activity: ApActivity = match raw.clone().try_into() {
        Ok(activity) => activity,
        Err(e) => {
            let conn = state
                .db_pool
                .get()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            create_unprocessable(&conn, (raw, Some(format!("{e:#?}"))).into()).await;
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    if activity.is_delete() && signed.deferred().is_some() {
        return Ok(StatusCode::ACCEPTED);
    }

    let is_authorized = if let Some(deferred) = signed.deferred() {
        let conn = state
            .db_pool
            .get()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        matches!(
            verify(&conn, deferred).await,
            Ok(VerificationType::Remote(_))
        )
    } else {
        signed.any()
    };

    if is_authorized {
        let conn = state
            .db_pool
            .get()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
        retrieve::activities(
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
