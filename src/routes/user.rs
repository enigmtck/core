use crate::{
    activity_pub::{ActorsPage, ApActor, ApCollection, FollowersPage, LeadersPage},
    db::Db,
    fairings::signatures::Signed,
    models::{
        followers::get_followers_by_profile_id, leaders::get_leaders_by_profile_id,
        profiles::get_profile_by_username,
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
    match get_profile_by_username((&conn).into(), username).await {
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
    match get_profile_by_username((&conn).into(), username).await {
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
    if let Some(_profile) = get_profile_by_username((&conn).into(), username).await {
        Ok(ActivityJson(Json(ApCollection::default())))
    } else {
        Err(Status::NotFound)
    }
}

#[get("/user/<username>/followers")]
pub async fn get_followers(
    signed: Signed,
    conn: Db,
    username: String,
) -> Result<ActivityJson<ApCollection>, Status> {
    if signed.local() {
        if let Some(profile) = get_profile_by_username((&conn).into(), username.clone()).await {
            let followers = get_followers_by_profile_id(Some(&conn), profile.id).await;

            Ok(ActivityJson(Json(ApCollection::from(ActorsPage {
                page: 0,
                profile,
                actors: followers
                    .iter()
                    .filter_map(|(_, remote_actor)| {
                        remote_actor
                            .as_ref()
                            .map(|remote_actor| remote_actor.clone().into())
                    })
                    .collect(),
            }))))
        } else if let Some(profile) = get_profile_by_username((&conn).into(), username).await {
            let followers = get_followers_by_profile_id(Some(&conn), profile.id).await;

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
    } else {
        Err(Status::NotFound)
    }
}

#[get("/user/<username>/following")]
pub async fn get_leaders(
    signed: Signed,
    conn: Db,
    username: String,
) -> Result<ActivityJson<ApCollection>, Status> {
    if signed.local() {
        if let Some(profile) = get_profile_by_username((&conn).into(), username.clone()).await {
            let leaders = get_leaders_by_profile_id(&conn, profile.id).await;

            Ok(ActivityJson(Json(ApCollection::from(ActorsPage {
                page: 0,
                profile,
                actors: leaders
                    .iter()
                    .filter_map(|(_, remote_actor)| {
                        remote_actor
                            .as_ref()
                            .map(|remote_actor| remote_actor.clone().into())
                    })
                    .collect(),
            }))))
        } else if let Some(profile) = get_profile_by_username((&conn).into(), username).await {
            let leaders = get_leaders_by_profile_id(&conn, profile.id).await;

            Ok(ActivityJson(Json(ApCollection::from(LeadersPage {
                page: 0,
                profile,
                leaders: leaders.iter().map(|(leader, _)| leader.clone()).collect(),
            }))))
        } else {
            Err(Status::NotFound)
        }
    } else {
        Err(Status::NotFound)
    }
}
