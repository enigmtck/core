use crate::activity_pub::retriever::{
    get_actor, get_ap_id_from_webfinger, get_object, get_remote_collection,
    get_remote_collection_page,
};
use crate::activity_pub::{ApActor, ApObject};
use crate::db::Db;
use crate::fairings::access_control::BlockList;
use crate::helper::{get_domain_from_url, get_domain_from_webfinger};
use crate::models::actors::{get_actor_by_username, get_actor_by_webfinger};
use rocket::http::Status;
use rocket::{get, serde::json::Json};

use crate::fairings::signatures::Signed;

/// This accepts an actor in URL form (e.g., https://enigmatick.social/user/justin).
#[get("/api/remote/webfinger?<id>")]
pub async fn remote_id(blocks: BlockList, conn: Db, id: &str) -> Result<String, Status> {
    let id = urlencoding::decode(id).map_err(|e| {
        log::error!("FAILED TO DECODE id: {e:#?}");
        Status::InternalServerError
    })?;
    let id = (*id).to_string();

    if blocks.is_blocked(get_domain_from_url(id.clone())) {
        Err(Status::Forbidden)
    } else {
        get_actor(&conn, id, None, true)
            .await
            .map(|actor| actor.get_webfinger().unwrap_or_default())
            .map_err(|e| {
                log::error!("FAILED TO RETRIEVE WEBFINGER: {e:#?}");
                Status::NotFound
            })
    }
}

/// This accepts an actor in URL form (e.g., https://enigmatick.social/user/justin).
#[get("/api/user/<username>/remote/webfinger?<id>")]
pub async fn remote_id_authenticated(
    blocks: BlockList,
    signed: Signed,
    conn: Db,
    username: &str,
    id: &str,
) -> Result<String, Status> {
    let id = urlencoding::decode(id).map_err(|e| {
        log::error!("FAILED TO DECODE id: {e:#?}");
        Status::BadRequest
    })?;
    let id = (*id).to_string();

    if blocks.is_blocked(get_domain_from_url(id.clone())) {
        Err(Status::Forbidden)
    } else if signed.local() {
        let profile = get_actor_by_username(&conn, username.to_string())
            .await
            .ok_or(Status::NotFound)?;

        get_actor(&conn, id, Some(profile), true)
            .await
            .map(|actor| actor.get_webfinger().unwrap_or_default())
            .map_err(|e| {
                log::error!("FAILED TO RETRIEVE WEBFINGER: {e:#?}");
                Status::NotFound
            })
    } else {
        log::error!("BAD SIGNATURE");
        Err(Status::Unauthorized)
    }
}

async fn remote_actor_response(conn: &Db, webfinger: String) -> Result<Json<ApActor>, Status> {
    if let Some(actor) = get_actor_by_webfinger(conn, webfinger.clone()).await {
        log::debug!("FOUND REMOTE ACTOR LOCALLY");
        Ok(Json(ApActor::from(actor)))
    } else if let Some(ap_id) = get_ap_id_from_webfinger(webfinger).await {
        log::debug!("RETRIEVING ACTOR WEBFINGER FROM REMOTE OR LOCAL PROFILE");
        if let Ok(actor) = get_actor(conn, ap_id, None, true).await {
            Ok(Json(actor))
        } else {
            log::error!("FAILED TO RETRIEVE ACTOR BY AP_ID");
            Err(Status::NotFound)
        }
    } else {
        log::error!("FAILED TO RETRIEVE ACTOR FROM DATABASE BY WEBFINGER");
        Err(Status::BadRequest)
    }
}

/// This accepts an actor in webfinger form (e.g., justin@enigmatick.social).
#[get("/api/remote/actor?<webfinger>")]
pub async fn remote_actor(
    blocks: BlockList,
    conn: Db,
    webfinger: &str,
) -> Result<Json<ApActor>, Status> {
    if blocks.is_blocked(get_domain_from_webfinger(webfinger.to_string())) {
        Err(Status::Forbidden)
    } else {
        remote_actor_response(&conn, webfinger.to_string()).await
    }
}

async fn remote_actor_authenticated_response(
    signed: Signed,
    conn: &Db,
    webfinger: String,
) -> Result<Json<ApActor>, Status> {
    if let Some(profile) = signed.profile() {
        let ap_id = get_ap_id_from_webfinger(webfinger).await.ok_or_else(|| {
            log::error!("Failed to get ActivityPub ID from Webfinger");
            Status::NotFound
        })?;

        Ok(Json(
            get_actor(conn, ap_id, Some(profile), true)
                .await
                .map_err(|e| {
                    log::error!("Failed to retrieve Actor: {e:#?}");
                    Status::NotFound
                })?,
        ))
    } else {
        Err(Status::Unauthorized)
    }
}

#[get("/api/user/<_username>/remote/actor?<webfinger>")]
pub async fn remote_actor_authenticated(
    blocks: BlockList,
    signed: Signed,
    conn: Db,
    _username: &str,
    webfinger: &str,
) -> Result<Json<ApActor>, Status> {
    if blocks.is_blocked(get_domain_from_webfinger(webfinger.to_string())) {
        Err(Status::Forbidden)
    } else {
        remote_actor_authenticated_response(signed, &conn, webfinger.to_string()).await
    }
}

#[get("/api/remote/followers?<webfinger>&<page>")]
pub async fn remote_followers(
    blocks: BlockList,
    conn: Db,
    webfinger: &str,
    page: Option<&str>,
) -> Result<Json<ApObject>, Status> {
    if blocks.is_blocked(get_domain_from_webfinger(webfinger.to_string())) {
        Err(Status::Forbidden)
    } else if let Ok(Json(actor)) = remote_actor_response(&conn, webfinger.to_string()).await {
        let followers = actor.followers.ok_or_else(|| {
            log::error!("Actor MUST HAVE FOLLOWERS COLLECTION");
            Status::InternalServerError
        })?;

        if let Some(page) = page {
            let url = urlencoding::decode(page).map_err(|e| {
                log::error!("FAILED TO DECODE page: {e:#?}");
                Status::InternalServerError
            })?;
            let url = &(*url).to_string();

            if url.contains(&followers) {
                let collection = get_remote_collection_page(&conn, None, page.to_string())
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO RETRIEVE REMOTE CollectionPage: {e:#?}");
                        Status::InternalServerError
                    })?;

                Ok(Json(ApObject::Collection(collection)))
            } else {
                Err(Status::NoContent)
            }
        } else {
            let collection = get_remote_collection(&conn, None, followers)
                .await
                .map_err(|e| {
                    log::error!("FAILED TO RETRIEVE REMOTE Collection: {e:#?}");
                    Status::InternalServerError
                })?;
            Ok(Json(ApObject::Collection(collection)))
        }
    } else {
        Err(Status::InternalServerError)
    }
}

/// This function returns either an ApCollection or an ApCollectionPage wrapped in
/// an ApObject. The `followers` attribute in the actor is used either directly (for
/// the ApCollection) or in tandem with the page to confirm that the page is associated
/// with the actor for the ApCollectionPage. The `page` parameter is URL encoded because
/// it's the standard URL ID that ActivityPub uses for such things and includes characters
/// that would interfere with the match (`?`, `:`, `/`, and `=`);
#[get("/api/user/<_username>/remote/followers?<webfinger>&<page>")]
pub async fn remote_followers_authenticated(
    blocks: BlockList,
    signed: Signed,
    conn: Db,
    _username: &str,
    webfinger: &str,
    page: Option<&str>,
) -> Result<Json<ApObject>, Status> {
    if blocks.is_blocked(get_domain_from_webfinger(webfinger.to_string())) {
        Err(Status::Forbidden)
    } else if let Some(profile) = signed.profile() {
        if let Ok(Json(actor)) =
            remote_actor_authenticated_response(signed, &conn, webfinger.to_string()).await
        {
            let followers = actor.followers.ok_or(Status::InternalServerError)?;
            if let Some(page) = page {
                let url = urlencoding::decode(page).map_err(|e| {
                    log::error!("FAILED TO DECODE Page: {e:#?}");
                    Status::UnprocessableEntity
                })?;
                let url = &(*url).to_string();

                if url.contains(&followers) {
                    let collection =
                        get_remote_collection_page(&conn, Some(profile), page.to_string())
                            .await
                            .map_err(|e| {
                                log::error!("FAILED TO RETRIEVE REMOTE CollectionPage: {e:#?}");
                                Status::InternalServerError
                            })?;
                    Ok(Json(ApObject::Collection(collection)))
                } else {
                    Err(Status::NoContent)
                }
            } else {
                let collection = get_remote_collection(&conn, Some(profile), followers)
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO RETRIEVE REMOTE Collection: {e:#?}");
                        Status::InternalServerError
                    })?;
                Ok(Json(ApObject::Collection(collection)))
            }
        } else {
            Err(Status::InternalServerError)
        }
    } else {
        Err(Status::Unauthorized)
    }
}

/// This function returns either an ApCollection or an ApCollectionPage wrapped in
/// an ApObject. The `followers` attribute in the actor is used either directly (for
/// the ApCollection) or in tandem with the page to confirm that the page is associated
/// with the actor for the ApCollectionPage. The `page` parameter is URL encoded because
/// it's the standard URL ID that ActivityPub uses for such things and includes characters
/// that would interfere with the match (`?`, `:`, `/`, and `=`);
#[get("/api/remote/following?<webfinger>&<page>")]
pub async fn remote_following(
    blocks: BlockList,
    conn: Db,
    webfinger: &str,
    page: Option<&str>,
) -> Result<Json<ApObject>, Status> {
    if blocks.is_blocked(get_domain_from_webfinger(webfinger.to_string())) {
        Err(Status::Forbidden)
    } else if let Ok(Json(actor)) = remote_actor_response(&conn, webfinger.to_string()).await {
        let following = actor.following.ok_or(Status::InternalServerError)?;
        if let Some(page) = page {
            let url = urlencoding::decode(page).map_err(|e| {
                log::error!("FAILED TO DECODE page: {e:#?}");
                Status::UnprocessableEntity
            })?;
            let url = &(*url).to_string();

            if url.contains(&following) {
                let collection = get_remote_collection_page(&conn, None, page.to_string())
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO RETRIEVE REMOTE CollectionPage: {e:#?}");
                        Status::InternalServerError
                    })?;
                Ok(Json(ApObject::Collection(collection)))
            } else {
                Err(Status::InternalServerError)
            }
        } else {
            let collection = get_remote_collection(&conn, None, following)
                .await
                .map_err(|e| {
                    log::error!("FAILED TO RETRIEVE REMOTE Collection: {e:#?}");
                    Status::InternalServerError
                })?;
            Ok(Json(ApObject::Collection(collection)))
        }
    } else {
        Err(Status::Unauthorized)
    }
}

#[get("/api/user/<_username>/remote/following?<webfinger>&<page>")]
pub async fn remote_following_authenticated(
    blocks: BlockList,
    signed: Signed,
    conn: Db,
    _username: &str,
    webfinger: &str,
    page: Option<&str>,
) -> Result<Json<ApObject>, Status> {
    if blocks.is_blocked(get_domain_from_webfinger(webfinger.to_string())) {
        Err(Status::Forbidden)
    } else if let Some(profile) = signed.profile() {
        if let Ok(Json(actor)) =
            remote_actor_authenticated_response(signed, &conn, webfinger.to_string()).await
        {
            let following = actor.following.ok_or(Status::InternalServerError)?;
            if let Some(page) = page {
                let url = urlencoding::decode(page).map_err(|e| {
                    log::error!("FAILED TO DECODE Page: {e:#?}");
                    Status::UnprocessableEntity
                })?;
                let url = &(*url).to_string();
                if url.contains(&following) {
                    let collection =
                        get_remote_collection_page(&conn, Some(profile), page.to_string())
                            .await
                            .map_err(|e| {
                                log::error!("FAILED TO RETRIEVE REMOTE CollectionPage: {e:#?}");
                                Status::InternalServerError
                            })?;

                    Ok(Json(ApObject::Collection(collection)))
                } else {
                    Err(Status::NoContent)
                }
            } else {
                let collection = get_remote_collection(&conn, Some(profile), following)
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO RETRIEVE REMOTE Collection: {e:#?}");
                        Status::InternalServerError
                    })?;
                Ok(Json(ApObject::Collection(collection)))
            }
        } else {
            Err(Status::NotFound)
        }
    } else {
        Err(Status::Unauthorized)
    }
}

/// This function returns either an ApCollection or an ApCollectionPage wrapped in
/// an ApObject. The `followers` attribute in the actor is used either directly (for
/// the ApCollection) or in tandem with the page to confirm that the page is associated
/// with the actor for the ApCollectionPage. The `page` parameter is URL encoded because
/// it's the standard URL ID that ActivityPub uses for such things and includes characters
/// that would interfere with the match (`?`, `:`, `/`, and `=`);
#[get("/api/remote/outbox?<webfinger>&<page>")]
pub async fn remote_outbox(
    blocks: BlockList,
    conn: Db,
    webfinger: &str,
    page: Option<&str>,
) -> Result<Json<ApObject>, Status> {
    if blocks.is_blocked(get_domain_from_webfinger(webfinger.to_string())) {
        Err(Status::Forbidden)
    } else if let Ok(Json(actor)) = remote_actor_response(&conn, webfinger.to_string()).await {
        if let Some(page) = page {
            let url = urlencoding::decode(page).map_err(|e| {
                log::error!("FAILED TO DECODE Page: {e:#?}");
                Status::UnprocessableEntity
            })?;
            let url = &(*url).to_string();
            if url.contains(&actor.outbox) {
                let collection = get_remote_collection_page(&conn, None, page.to_string())
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO RETRIEVE REMOTE CollectionPage: {e:#?}");
                        Status::InternalServerError
                    })?;

                Ok(Json(ApObject::Collection(collection)))
            } else {
                Err(Status::NoContent)
            }
        } else {
            let collection = get_remote_collection(&conn, None, actor.outbox)
                .await
                .map_err(|e| {
                    log::error!("FAILED TO RETRIEVE REMOTE Collection: {e:#?}");
                    Status::InternalServerError
                })?;
            Ok(Json(ApObject::Collection(collection)))
        }
    } else {
        Err(Status::new(520))
    }
}

#[get("/api/user/<_username>/remote/outbox?<webfinger>&<page>")]
pub async fn remote_outbox_authenticated(
    blocks: BlockList,
    signed: Signed,
    conn: Db,
    _username: &str,
    webfinger: &str,
    page: Option<&str>,
) -> Result<Json<ApObject>, Status> {
    if blocks.is_blocked(get_domain_from_webfinger(webfinger.to_string())) {
        Err(Status::Forbidden)
    } else if !signed.local() {
        Err(Status::Unauthorized)
    } else if let Ok(Json(actor)) = remote_actor_response(&conn, webfinger.to_string()).await {
        let profile = signed.profile();

        if let Some(page) = page {
            let url = urlencoding::decode(page).map_err(|e| {
                log::error!("FAILED TO DECODE Page: {e:#?}");
                Status::UnprocessableEntity
            })?;
            let url = &(*url).to_string();
            if url.contains(&actor.outbox) {
                let collection = get_remote_collection_page(&conn, profile, page.to_string())
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO RETRIEVE REMOTE CollectionPage: {e:#?}");
                        Status::InternalServerError
                    })?;

                Ok(Json(ApObject::Collection(collection)))
            } else {
                Err(Status::UnprocessableEntity)
            }
        } else {
            let collection = get_remote_collection(&conn, profile, actor.outbox)
                .await
                .map_err(|e| {
                    log::error!("FAILED TO RETRIEVE REMOTE Collection: {e:#?}");
                    Status::ServiceUnavailable
                })?;
            Ok(Json(ApObject::Collection(collection)))
        }
    } else {
        Err(Status::ServiceUnavailable)
    }
}

#[get("/api/user/<_username>/remote/keys?<webfinger>")]
pub async fn remote_keys_authenticated(
    blocks: BlockList,
    signed: Signed,
    conn: Db,
    _username: &str,
    webfinger: &str,
) -> Result<Json<ApObject>, Status> {
    if blocks.is_blocked(get_domain_from_webfinger(webfinger.to_string())) {
        Err(Status::Forbidden)
    } else if let Some(profile) = signed.profile() {
        let Json(actor) =
            remote_actor_authenticated_response(signed, &conn, webfinger.to_string()).await?;

        let keys = actor.keys.ok_or_else(|| {
            log::error!("Actor must have a Keys collection");
            Status::InternalServerError
        })?;

        let keys = format!("{keys}?otk=true");

        let collection = get_remote_collection(&conn, Some(profile), keys)
            .await
            .map_err(|e| {
                log::error!("FAILED TO RETRIEVE REMOTE Collection: {e:#?}");
                Status::InternalServerError
            })?;
        Ok(Json(ApObject::Collection(collection)))
    } else {
        Err(Status::Unauthorized)
    }
}

#[get("/api/remote/object?<id>")]
pub async fn remote_object(
    blocks: BlockList,
    signed: Signed,
    conn: Db,
    id: &str,
) -> Result<Json<ApObject>, Status> {
    if let Ok(url) = urlencoding::decode(id) {
        let url = &(*url).to_string();

        if blocks.is_blocked(get_domain_from_url(id.to_string())) {
            Err(Status::Forbidden)
        } else if let Some(object) = get_object(&conn, signed.profile(), url.to_string()).await {
            Ok(Json(object))
        } else {
            Err(Status::NotFound)
        }
    } else {
        Err(Status::UnprocessableEntity)
    }
}
