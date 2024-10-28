use crate::{
    activity_pub::{ApActor, ApCollection, FollowersPage, LeadersPage},
    db::Db,
    fairings::signatures::Signed,
    models::{
        actors::get_actor_by_username, followers::get_followers_by_actor_id,
        leaders::get_leaders_by_actor_id,
    },
};
use rocket::{get, http::Status, response::Redirect, serde::json::Json};

use super::{ActivityJson, LdJson};

#[get("/user/<username>", format = "text/html", rank = 1)]
pub async fn person_redirect(username: String) -> Redirect {
    log::debug!("REDIRECTING {username}");
    Redirect::to(format!("/@{username}"))
}

#[get("/user/<username>", format = "application/activity+json", rank = 2)]
pub async fn person_activity_json(
    signed: Signed,
    conn: Db,
    username: String,
) -> Result<ActivityJson<ApActor>, Status> {
    match get_actor_by_username(&conn, username).await {
        Some(profile) => {
            let actor = if signed.local() {
                ApActor::from(profile).load_ephemeral(&conn).await
            } else {
                ApActor::from(profile)
            };

            Ok(ActivityJson(Json(actor)))
        }
        None => Err(Status::NotFound),
    }
}

#[get("/user/<username>", format = "application/ld+json", rank = 3)]
pub async fn person_ld_json(
    signed: Signed,
    conn: Db,
    username: String,
) -> Result<LdJson<ApActor>, Status> {
    match get_actor_by_username(&conn, username).await {
        Some(profile) => {
            let actor = if signed.local() {
                ApActor::from(profile).load_ephemeral(&conn).await
            } else {
                ApActor::from(profile)
            };

            Ok(LdJson(Json(actor)))
        }
        None => Err(Status::NotFound),
    }
}

#[get("/user/<username>/liked")]
pub async fn liked_get(conn: Db, username: String) -> Result<ActivityJson<ApCollection>, Status> {
    if let Some(_profile) = get_actor_by_username(&conn, username).await {
        Ok(ActivityJson(Json(ApCollection::default())))
    } else {
        Err(Status::NotFound)
    }
}

#[get("/user/<username>/followers")]
pub async fn get_followers(
    _signed: Signed,
    conn: Db,
    username: String,
) -> Result<ActivityJson<ApCollection>, Status> {
    if let Some(profile) = get_actor_by_username(&conn, username).await {
        let followers = get_followers_by_actor_id(&conn, profile.id).await;

        Ok(ActivityJson(Json(ApCollection::from(FollowersPage {
            page: 0,
            profile,
            followers: followers
                .iter()
                .map(|(follower, _)| follower.clone())
                .collect(),
        }))))
    } else {
        Err(Status::NotFound)
    }
}

#[get("/user/<username>/following")]
pub async fn get_leaders(
    _signed: Signed,
    conn: Db,
    username: String,
) -> Result<ActivityJson<ApCollection>, Status> {
    if let Some(profile) = get_actor_by_username(&conn, username).await {
        let leaders = get_leaders_by_actor_id(&conn, profile.id).await;

        Ok(ActivityJson(Json(ApCollection::from(LeadersPage {
            page: 0,
            profile,
            leaders: leaders.iter().map(|(leader, _)| leader.clone()).collect(),
        }))))
    } else {
        Err(Status::NotFound)
    }
}
