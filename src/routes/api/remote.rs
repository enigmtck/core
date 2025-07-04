use crate::db::Db;
use crate::fairings::access_control::BlockList;
use crate::helper::{get_domain_from_url, get_domain_from_webfinger};
use crate::models::actors::{get_actor_by_username, get_actor_by_webfinger, Actor};
use crate::retriever::{
    get_actor, get_ap_id_from_webfinger, get_object, get_remote_collection,
    get_remote_collection_page,
};
use crate::GetWebfinger;
use crate::LoadEphemeral;
use jdt_activity_pub::{ApActor, ApObject};
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

    if blocks.is_blocked(get_domain_from_url(id.clone()).ok_or(Status::InternalServerError)?) {
        Err(Status::Forbidden)
    } else {
        let actor = get_actor(Some(&conn), id, None, true).await.map_err(|e| {
            log::error!("Failed to retrieve Actor: {e:#?}");
            Status::NotFound
        })?;
        actor.get_webfinger().await.ok_or(Status::NotFound)
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

    if blocks.is_blocked(get_domain_from_url(id.clone()).ok_or(Status::InternalServerError)?) {
        Err(Status::Forbidden)
    } else if signed.local() {
        let profile = get_actor_by_username(Some(&conn), username.to_string())
            .await
            .map_err(|_| Status::NotFound)?;

        let actor = get_actor(Some(&conn), id, Some(profile), true)
            .await
            .map_err(|e| {
                log::error!("Failed to retrieve Actor: {e:#?}");
                Status::NotFound
            })?;
        actor.get_webfinger().await.ok_or(Status::NotFound)
    } else {
        log::error!("BAD SIGNATURE");
        Err(Status::Unauthorized)
    }
}

async fn remote_actor_response(
    conn: &Db,
    webfinger: String,
    requester: Option<Actor>,
) -> Result<Json<ApActor>, Status> {
    if let Ok(actor) = get_actor_by_webfinger(Some(conn), webfinger.clone()).await {
        log::debug!("FOUND REMOTE ACTOR LOCALLY");
        Ok(Json(
            ApActor::from(actor).load_ephemeral(conn, requester).await,
        ))
    } else if let Ok(ap_id) = get_ap_id_from_webfinger(webfinger).await {
        log::debug!("RETRIEVING ACTOR WEBFINGER FROM REMOTE OR LOCAL PROFILE");
        if let Ok(actor) = get_actor(Some(conn), ap_id, None, true).await {
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
    signed: Signed,
    conn: Db,
    webfinger: &str,
) -> Result<Json<ApActor>, Status> {
    if blocks.is_blocked(get_domain_from_webfinger(webfinger.to_string())) {
        Err(Status::Forbidden)
    } else {
        remote_actor_response(&conn, webfinger.to_string(), signed.profile()).await
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
    remote_actor(blocks, signed, conn, webfinger).await
}

#[get("/api/remote/followers?<webfinger>&<page>")]
pub async fn remote_followers(
    blocks: BlockList,
    signed: Signed,
    conn: Db,
    webfinger: &str,
    page: Option<&str>,
) -> Result<Json<ApObject>, Status> {
    if blocks.is_blocked(get_domain_from_webfinger(webfinger.to_string())) {
        Err(Status::Forbidden)
    } else if let Ok(Json(actor)) =
        remote_actor_response(&conn, webfinger.to_string(), signed.profile()).await
    {
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
            remote_actor_response(&conn, webfinger.to_string(), signed.profile()).await
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
    signed: Signed,
    conn: Db,
    webfinger: &str,
    page: Option<&str>,
) -> Result<Json<ApObject>, Status> {
    if blocks.is_blocked(get_domain_from_webfinger(webfinger.to_string())) {
        Err(Status::Forbidden)
    } else if let Ok(Json(actor)) =
        remote_actor_response(&conn, webfinger.to_string(), signed.profile()).await
    {
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
            remote_actor_response(&conn, webfinger.to_string(), signed.profile()).await
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
    signed: Signed,
    conn: Db,
    webfinger: &str,
    page: Option<&str>,
) -> Result<Json<ApObject>, Status> {
    if blocks.is_blocked(get_domain_from_webfinger(webfinger.to_string())) {
        Err(Status::Forbidden)
    } else if let Ok(Json(actor)) =
        remote_actor_response(&conn, webfinger.to_string(), signed.profile()).await
    {
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
    } else if let Ok(Json(actor)) =
        remote_actor_response(&conn, webfinger.to_string(), signed.profile()).await
    {
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
            remote_actor_response(&conn, webfinger.to_string(), signed.profile()).await?;

        let keys = actor.keys.ok_or_else(|| {
            log::error!("Actor must have a Keys collection");
            Status::InternalServerError
        })?;

        let keys = format!("{keys}?mkp=true");

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

        if blocks
            .is_blocked(get_domain_from_url(id.to_string()).ok_or(Status::InternalServerError)?)
        {
            Err(Status::Forbidden)
        } else if let Ok(object) = get_object(Some(&conn), signed.profile(), url.to_string()).await
        {
            Ok(Json(object))
        } else {
            Err(Status::NotFound)
        }
    } else {
        Err(Status::UnprocessableEntity)
    }
}
