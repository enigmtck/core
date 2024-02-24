use crate::activity_pub::retriever::{
    get_actor, get_ap_id_from_webfinger, get_note, get_remote_collection,
    get_remote_collection_page,
};
use crate::activity_pub::{ApActor, ApNote, ApObject};
use crate::db::Db;
use crate::models::remote_actors::get_remote_actor_by_webfinger;
use rocket::http::Status;
use rocket::{get, serde::json::Json};

use crate::fairings::signatures::Signed;
use crate::models::profiles::get_profile_by_username;
use crate::signing::VerificationType;

/// This accepts an actor in URL form (e.g., https://enigmatick.social/user/justin).
#[get("/api/remote/webfinger?<id>")]
pub async fn remote_id(conn: Db, id: String) -> Result<String, Status> {
    if let Ok(id) = urlencoding::decode(&id) {
        let id = (*id).to_string();
        if let Some(actor) = get_actor(&conn, id, None, true).await {
            if let Some(webfinger) = actor.get_webfinger() {
                Ok(webfinger)
            } else {
                Err(Status::NotFound)
            }
        } else {
            Err(Status::NotFound)
        }
    } else {
        log::error!("FAILED TO URL DECODE ID");
        Err(Status::BadRequest)
    }
}

/// This accepts an actor in URL form (e.g., https://enigmatick.social/user/justin).
#[get("/api/user/<username>/remote/webfinger?<id>")]
pub async fn remote_id_authenticated(
    signed: Signed,
    conn: Db,
    username: String,
    id: String,
) -> Result<String, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Some(profile) = get_profile_by_username((&conn).into(), username).await {
            if let Ok(id) = urlencoding::decode(&id) {
                let id = (*id).to_string();
                if let Some(actor) = get_actor(&conn, id, Some(profile), true).await {
                    if let Some(webfinger) = actor.get_webfinger() {
                        Ok(webfinger)
                    } else {
                        Err(Status::NotFound)
                    }
                } else {
                    Err(Status::NotFound)
                }
            } else {
                log::error!("FAILED TO URL DECODE ID");
                Err(Status::BadRequest)
            }
        } else {
            log::error!("FAILED TO GET REQUESTER PROFILE");
            Err(Status::Forbidden)
        }
    } else {
        log::error!("BAD SIGNATURE");
        Err(Status::Forbidden)
    }
}

async fn remote_actor_response(conn: &Db, webfinger: String) -> Result<Json<ApActor>, Status> {
    if let Some(actor) = get_remote_actor_by_webfinger(conn, webfinger.clone()).await {
        log::debug!("FOUND REMOTE ACTOR LOCALLY");
        Ok(Json(ApActor::from(actor)))
    } else if let Some(ap_id) = get_ap_id_from_webfinger(webfinger).await {
        log::debug!("RETRIEVING ACTOR WEBFINGER FROM REMOTE OR LOCAL PROFILE");
        if let Some(actor) = get_actor(conn, ap_id, None, true).await {
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
pub async fn remote_actor(conn: Db, webfinger: String) -> Result<Json<ApActor>, Status> {
    remote_actor_response(&conn, webfinger).await
}

async fn remote_actor_authenticated_response(
    signed: Signed,
    conn: &Db,
    username: String,
    webfinger: String,
) -> Result<Json<ApActor>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Some(profile) = get_profile_by_username(conn.into(), username).await {
            if let Some(ap_id) = get_ap_id_from_webfinger(webfinger).await {
                log::debug!("RETRIEVING ACTOR WEBFINGER FROM REMOTE OR LOCAL PROFILE");
                if let Some(actor) = get_actor(conn, ap_id, Some(profile), true).await {
                    Ok(Json(actor))
                } else {
                    log::error!("FAILED TO RETRIEVE ACTOR BY AP_ID");
                    Err(Status::NotFound)
                }
            } else {
                Err(Status::NotFound)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[get("/api/user/<username>/remote/actor?<webfinger>")]
pub async fn remote_actor_authenticated(
    signed: Signed,
    conn: Db,
    username: String,
    webfinger: String,
) -> Result<Json<ApActor>, Status> {
    remote_actor_authenticated_response(signed, &conn, username, webfinger).await
}

#[get("/api/remote/followers?<webfinger>&<page>")]
pub async fn remote_followers(
    conn: Db,
    webfinger: String,
    page: Option<String>,
) -> Result<Json<ApObject>, Status> {
    if let Ok(Json(actor)) = remote_actor_response(&conn, webfinger).await {
        if let Some(page) = page {
            if let Ok(url) = urlencoding::decode(&page) {
                let url = &(*url).to_string();
                if let Some(followers) = actor.followers.clone() {
                    if url.contains(&followers) {
                        if let Some(collection) =
                            get_remote_collection_page(&conn, None, page).await
                        {
                            Ok(Json(ApObject::CollectionPage(collection)))
                        } else {
                            Err(Status::NoContent)
                        }
                    } else {
                        Err(Status::NoContent)
                    }
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else if let Some(followers) = actor.followers {
            if let Some(collection) = get_remote_collection(&conn, None, followers).await {
                Ok(Json(ApObject::Collection(collection)))
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

/// This function returns either an ApCollection or an ApCollectionPage wrapped in
/// an ApObject. The `followers` attribute in the actor is used either directly (for
/// the ApCollection) or in tandem with the page to confirm that the page is associated
/// with the actor for the ApCollectionPage. The `page` parameter is URL encoded because
/// it's the standard URL ID that ActivityPub uses for such things and includes characters
/// that would interfere with the match (`?`, `:`, `/`, and `=`);
#[get("/api/user/<username>/remote/followers?<webfinger>&<page>")]
pub async fn remote_followers_authenticated(
    signed: Signed,
    conn: Db,
    username: String,
    webfinger: String,
    page: Option<String>,
) -> Result<Json<ApObject>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Some(profile) = get_profile_by_username((&conn).into(), username.clone()).await {
            if let Ok(Json(actor)) =
                remote_actor_authenticated_response(signed, &conn, username, webfinger).await
            {
                if let Some(page) = page {
                    if let Ok(url) = urlencoding::decode(&page) {
                        let url = &(*url).to_string();
                        if let Some(followers) = actor.followers.clone() {
                            if url.contains(&followers) {
                                if let Some(collection) =
                                    get_remote_collection_page(&conn, Some(profile), page).await
                                {
                                    Ok(Json(ApObject::CollectionPage(collection)))
                                } else {
                                    Err(Status::NoContent)
                                }
                            } else {
                                Err(Status::NoContent)
                            }
                        } else {
                            Err(Status::NoContent)
                        }
                    } else {
                        Err(Status::NoContent)
                    }
                } else if let Some(followers) = actor.followers {
                    if let Some(collection) =
                        get_remote_collection(&conn, Some(profile), followers).await
                    {
                        Ok(Json(ApObject::Collection(collection)))
                    } else {
                        Err(Status::NoContent)
                    }
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
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
    conn: Db,
    webfinger: String,
    page: Option<String>,
) -> Result<Json<ApObject>, Status> {
    if let Ok(Json(actor)) = remote_actor_response(&conn, webfinger).await {
        if let Some(page) = page {
            if let Ok(url) = urlencoding::decode(&page) {
                let url = &(*url).to_string();
                if let Some(following) = actor.following.clone() {
                    if url.contains(&following) {
                        if let Some(collection) =
                            get_remote_collection_page(&conn, None, page).await
                        {
                            Ok(Json(ApObject::CollectionPage(collection)))
                        } else {
                            Err(Status::NoContent)
                        }
                    } else {
                        Err(Status::NoContent)
                    }
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else if let Some(following) = actor.following {
            if let Some(collection) = get_remote_collection(&conn, None, following).await {
                Ok(Json(ApObject::Collection(collection)))
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[get("/api/user/<username>/remote/following?<webfinger>&<page>")]
pub async fn remote_following_authenticated(
    signed: Signed,
    conn: Db,
    username: String,
    webfinger: String,
    page: Option<String>,
) -> Result<Json<ApObject>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Some(profile) = get_profile_by_username((&conn).into(), username.clone()).await {
            if let Ok(Json(actor)) =
                remote_actor_authenticated_response(signed, &conn, username, webfinger).await
            {
                if let Some(page) = page {
                    if let Ok(url) = urlencoding::decode(&page) {
                        let url = &(*url).to_string();
                        if let Some(following) = actor.following.clone() {
                            if url.contains(&following) {
                                if let Some(collection) =
                                    get_remote_collection_page(&conn, Some(profile), page).await
                                {
                                    Ok(Json(ApObject::CollectionPage(collection)))
                                } else {
                                    Err(Status::NoContent)
                                }
                            } else {
                                Err(Status::NoContent)
                            }
                        } else {
                            Err(Status::NoContent)
                        }
                    } else {
                        Err(Status::NoContent)
                    }
                } else if let Some(following) = actor.following {
                    if let Some(collection) =
                        get_remote_collection(&conn, Some(profile), following).await
                    {
                        Ok(Json(ApObject::Collection(collection)))
                    } else {
                        Err(Status::NoContent)
                    }
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
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
    conn: Db,
    webfinger: String,
    page: Option<String>,
) -> Result<Json<ApObject>, Status> {
    if let Ok(Json(actor)) = remote_actor_response(&conn, webfinger).await {
        if let Some(page) = page {
            if let Ok(url) = urlencoding::decode(&page) {
                let url = &(*url).to_string();
                if url.contains(&actor.outbox) {
                    if let Some(collection) = get_remote_collection_page(&conn, None, page).await {
                        Ok(Json(ApObject::CollectionPage(collection)))
                    } else {
                        Err(Status::NoContent)
                    }
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else if let Some(collection) = get_remote_collection(&conn, None, actor.outbox).await {
            Ok(Json(ApObject::Collection(collection)))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[get("/api/user/<username>/remote/outbox?<webfinger>&<page>")]
pub async fn remote_outbox_authenticated(
    signed: Signed,
    conn: Db,
    username: String,
    webfinger: String,
    page: Option<String>,
) -> Result<Json<ApObject>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Some(profile) = get_profile_by_username((&conn).into(), username).await {
            if let Ok(Json(actor)) = remote_actor_response(&conn, webfinger).await {
                if let Some(page) = page {
                    if let Ok(url) = urlencoding::decode(&page) {
                        let url = &(*url).to_string();
                        if url.contains(&actor.outbox) {
                            if let Some(collection) =
                                get_remote_collection_page(&conn, Some(profile), page).await
                            {
                                Ok(Json(ApObject::CollectionPage(collection)))
                            } else {
                                Err(Status::NoContent)
                            }
                        } else {
                            Err(Status::NoContent)
                        }
                    } else {
                        Err(Status::NoContent)
                    }
                } else if let Some(collection) =
                    get_remote_collection(&conn, Some(profile), actor.outbox).await
                {
                    Ok(Json(ApObject::Collection(collection)))
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[get("/api/remote/note?<id>")]
pub async fn remote_note(conn: Db, id: String) -> Result<Json<ApNote>, Status> {
    if let Ok(url) = urlencoding::decode(&id) {
        let url = &(*url).to_string();
        if let Some(note) = get_note(&conn, None, url.to_string()).await {
            Ok(Json(note))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[get("/api/user/<username>/remote/note?<id>")]
pub async fn remote_note_authenticated(
    signed: Signed,
    conn: Db,
    username: String,
    id: String,
) -> Result<Json<ApNote>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Some(profile) = get_profile_by_username((&conn).into(), username).await {
            if let Some(note) = get_note(&conn, Some(profile), id).await {
                log::debug!("{note:#?}");
                Ok(Json(note))
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}
