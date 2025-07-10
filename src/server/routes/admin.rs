use crate::{
    admin::{self, NewUser},
    models::actors::{
        get_actor_by_as_id, get_muted_terms_by_username, guaranteed_actor,
        update_muted_terms_by_username, Actor,
    },
    retriever::get_actor,
    server::{extractors::AxumSigned, AppState},
};
use axum::{
    extract::{rejection::JsonRejection, ConnectInfo, Path, State},
    http::StatusCode,
    Json,
};
use jdt_activity_pub::{ApActor, ApFollow, MaybeMultiple};
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MutedTermsActionType {
    Add,
    Remove,
}

#[derive(Deserialize)]
pub struct MutedTermsAction {
    pub action: MutedTermsActionType,
    pub terms: Vec<String>,
}

pub async fn create_user(
    State(state): State<AppState>,
    user: Result<Json<NewUser>, JsonRejection>,
) -> Result<Json<Actor>, StatusCode> {
    if !*crate::REGISTRATION_ENABLED {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let user = user.map_err(|_| StatusCode::BAD_REQUEST)?.0;
    log::debug!("AXUM CREATING USER\n{user:#?}");

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    admin::create_user(&conn, user)
        .await
        .map(Json)
        .map_err(|_| StatusCode::NO_CONTENT)
}

pub async fn relay_post(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    actor_id: String,
) -> Result<StatusCode, StatusCode> {
    if !addr.ip().is_loopback() {
        return Err(StatusCode::FORBIDDEN);
    }

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let profile = guaranteed_actor(&conn, None).await;

    let actor = if let Ok(actor) = get_actor_by_as_id(&conn, actor_id.clone()).await {
        Some(ApActor::from(actor))
    } else {
        (get_actor(&conn, actor_id, None, true).await).ok()
    };

    let inbox = if let Some(actor) = actor.clone() {
        if let Some(endpoints) = actor.endpoints {
            Some(endpoints.shared_inbox)
        } else {
            Some(actor.inbox)
        }
    } else {
        None
    };

    if let (Some(_inbox), Some(actor)) = (inbox, actor) {
        let _follow = ApFollow {
            actor: profile.as_id.into(),
            to: MaybeMultiple::Single(actor.id.unwrap_or_default()),
            ..Default::default()
        };
        // The original logic doesn't do anything with `_follow`, so we replicate that.
    }

    Ok(StatusCode::ACCEPTED)
}

pub async fn get_muted_terms(
    State(state): State<AppState>,
    signed: AxumSigned,
    Path(username): Path<String>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let profile = signed.profile().ok_or(StatusCode::UNAUTHORIZED)?;

    if profile.ek_username != Some(username.clone()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    get_muted_terms_by_username(&conn, username)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn manage_muted_terms(
    State(state): State<AppState>,
    signed: AxumSigned,
    Path(username): Path<String>,
    action: Result<Json<MutedTermsAction>, JsonRejection>,
) -> Result<StatusCode, StatusCode> {
    let profile = signed.profile().ok_or(StatusCode::UNAUTHORIZED)?;

    if profile.ek_username != Some(username.clone()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let action = action.map_err(|_| StatusCode::BAD_REQUEST)?.0;

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut all_terms = get_muted_terms_by_username(&conn, username.clone())
        .await
        .unwrap_or_default();

    match action.action {
        MutedTermsActionType::Add => {
            for term in action.terms {
                if !all_terms.contains(&term) {
                    all_terms.push(term);
                }
            }
        }
        MutedTermsActionType::Remove => {
            all_terms.retain(|term| !action.terms.contains(term));
        }
    }

    update_muted_terms_by_username(&conn, username, all_terms)
        .await
        .map(|_| StatusCode::OK)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
