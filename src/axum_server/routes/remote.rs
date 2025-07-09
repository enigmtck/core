use crate::{
    axum_server::{extractors::AxumSigned, AppState},
    db::runner::DbRunner,
    helper::{get_domain_from_url, get_domain_from_webfinger},
    models::actors::{get_actor_by_webfinger, Actor},
    retriever::{
        get_actor, get_ap_id_from_webfinger, get_object, get_remote_collection,
        get_remote_collection_page,
    },
    GetWebfinger, LoadEphemeral,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use jdt_activity_pub::{ApActor, ApObject};
use serde::Deserialize;

// Helper function to reduce repetition
async fn remote_actor_logic(
    conn: &impl DbRunner,
    webfinger: String,
    requester: Option<Actor>,
) -> Result<Json<ApActor>, StatusCode> {
    if let Ok(actor) = get_actor_by_webfinger(conn, webfinger.clone()).await {
        log::debug!("FOUND REMOTE ACTOR LOCALLY");
        Ok(Json(
            ApActor::from(actor).load_ephemeral(conn, requester).await,
        ))
    } else if let Ok(ap_id) = get_ap_id_from_webfinger(webfinger).await {
        log::debug!("RETRIEVING ACTOR WEBFINGER FROM REMOTE OR LOCAL PROFILE");
        if let Ok(actor) = get_actor(conn, ap_id, requester, true).await {
            Ok(Json(actor))
        } else {
            log::error!("FAILED TO RETRIEVE ACTOR BY AP_ID");
            Err(StatusCode::NOT_FOUND)
        }
    } else {
        log::error!("FAILED TO RETRIEVE ACTOR FROM DATABASE BY WEBFINGER");
        Err(StatusCode::BAD_REQUEST)
    }
}

#[derive(Deserialize)]
pub struct WebfingerQuery {
    pub webfinger: String,
}

#[derive(Deserialize)]
pub struct IdQuery {
    pub id: String,
}

#[derive(Deserialize)]
pub struct PageQuery {
    pub webfinger: String,
    pub page: Option<String>,
}

pub async fn remote_actor(
    State(state): State<AppState>,
    signed: AxumSigned,
    Query(query): Query<WebfingerQuery>,
) -> Result<Json<ApActor>, StatusCode> {
    if state
        .block_list
        .is_blocked(get_domain_from_webfinger(query.webfinger.clone()))
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    remote_actor_logic(&conn, query.webfinger, signed.profile()).await
}

pub async fn remote_webfinger_by_id(
    State(state): State<AppState>,
    signed: AxumSigned,
    Query(query): Query<IdQuery>,
) -> Result<String, StatusCode> {
    let id = urlencoding::decode(&query.id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let id = id.to_string();

    let domain = get_domain_from_url(id.clone()).ok_or(StatusCode::BAD_REQUEST)?;
    if state.block_list.is_blocked(domain) {
        return Err(StatusCode::FORBIDDEN);
    }

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let actor = get_actor(&conn, id, signed.profile(), true)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    actor.get_webfinger().await.ok_or(StatusCode::NOT_FOUND)
}

pub async fn remote_followers(
    State(state): State<AppState>,
    signed: AxumSigned,
    Query(query): Query<PageQuery>,
) -> Result<Json<ApObject>, StatusCode> {
    if state
        .block_list
        .is_blocked(get_domain_from_webfinger(query.webfinger.clone()))
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Json(actor) = remote_actor_logic(&conn, query.webfinger, signed.profile()).await?;
    let followers_url = actor.followers.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(page) = query.page {
        let decoded_page_url = urlencoding::decode(&page).map_err(|_| StatusCode::BAD_REQUEST)?;
        if !decoded_page_url.contains(&followers_url) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let collection = get_remote_collection_page(&conn, signed.profile(), page)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(ApObject::Collection(collection)))
    } else {
        let collection = get_remote_collection(&conn, signed.profile(), followers_url)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(ApObject::Collection(collection)))
    }
}

pub async fn remote_following(
    State(state): State<AppState>,
    signed: AxumSigned,
    Query(query): Query<PageQuery>,
) -> Result<Json<ApObject>, StatusCode> {
    if state
        .block_list
        .is_blocked(get_domain_from_webfinger(query.webfinger.clone()))
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Json(actor) = remote_actor_logic(&conn, query.webfinger, signed.profile()).await?;
    let following_url = actor.following.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(page) = query.page {
        let decoded_page_url = urlencoding::decode(&page).map_err(|_| StatusCode::BAD_REQUEST)?;
        if !decoded_page_url.contains(&following_url) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let collection = get_remote_collection_page(&conn, signed.profile(), page)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(ApObject::Collection(collection)))
    } else {
        let collection = get_remote_collection(&conn, signed.profile(), following_url)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(ApObject::Collection(collection)))
    }
}

pub async fn remote_outbox(
    State(state): State<AppState>,
    signed: AxumSigned,
    Query(query): Query<PageQuery>,
) -> Result<Json<ApObject>, StatusCode> {
    if state
        .block_list
        .is_blocked(get_domain_from_webfinger(query.webfinger.clone()))
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Json(actor) = remote_actor_logic(&conn, query.webfinger, signed.profile()).await?;
    let outbox_url = actor.outbox;

    if let Some(page) = query.page {
        log::debug!("{page:?}");
        let decoded_page_url = urlencoding::decode(&page).map_err(|_| StatusCode::BAD_REQUEST)?;
        if !decoded_page_url.contains(&outbox_url) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let collection = get_remote_collection_page(&conn, signed.profile(), page)
            .await
            .map_err(|e| {
                log::error!("{e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        Ok(Json(ApObject::Collection(collection)))
    } else {
        let collection = get_remote_collection(&conn, signed.profile(), outbox_url)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(ApObject::Collection(collection)))
    }
}

pub async fn remote_keys(
    State(state): State<AppState>,
    signed: AxumSigned,
    Query(query): Query<WebfingerQuery>,
) -> Result<Json<ApObject>, StatusCode> {
    if state
        .block_list
        .is_blocked(get_domain_from_webfinger(query.webfinger.clone()))
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let profile = signed.profile().ok_or(StatusCode::UNAUTHORIZED)?;

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Json(actor) = remote_actor_logic(&conn, query.webfinger, Some(profile.clone())).await?;
    let keys_url = actor.keys.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let keys_url_with_param = format!("{keys_url}?mkp=true");

    let collection = get_remote_collection(&conn, Some(profile), keys_url_with_param)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ApObject::Collection(collection)))
}

pub async fn remote_object(
    State(state): State<AppState>,
    signed: AxumSigned,
    Query(query): Query<IdQuery>,
) -> Result<Json<ApObject>, StatusCode> {
    let url = urlencoding::decode(&query.id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let url = url.to_string();

    let domain = get_domain_from_url(url.clone()).ok_or(StatusCode::BAD_REQUEST)?;
    if state.block_list.is_blocked(domain) {
        return Err(StatusCode::FORBIDDEN);
    }

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let object = get_object(&conn, signed.profile(), url)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(object))
}
