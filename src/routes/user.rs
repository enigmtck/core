use crate::{
    activity_pub::{
        retriever::get_actor, ActorsPage, ApActor, ApCollection, FollowersPage, LeadersPage,
    },
    db::{get_leaders_by_profile_id, Db},
    fairings::signatures::Signed,
    models::{
        followers::get_followers_by_profile_id, leaders::Leader, profiles::get_profile_by_username,
    },
    signing::VerificationType,
};
use rocket::{futures::future::join_all, get, http::Status, response::Redirect, serde::json::Json};

#[get("/user/<username>", format = "text/html", rank = 1)]
pub async fn person_redirect(username: String) -> Redirect {
    log::debug!("REDIRECTING {username}");
    Redirect::to(format!("/@{username}"))
}

#[get("/user/<username>", rank = 2)]
pub async fn person(conn: Db, username: String) -> Result<Json<ApActor>, Status> {
    match get_profile_by_username(&conn, username).await {
        Some(profile) => Ok(Json(ApActor::from(profile))),
        None => Err(Status::NoContent),
    }
}

#[get("/user/<username>/liked")]
pub async fn liked_get(conn: Db, username: String) -> Result<Json<ApCollection>, Status> {
    if let Some(_profile) = get_profile_by_username(&conn, username).await {
        Ok(Json(ApCollection::default()))
    } else {
        Err(Status::NoContent)
    }
}

#[get("/user/<username>/followers")]
pub async fn get_followers(
    signed: Signed,
    conn: Db,
    username: String,
) -> Result<Json<ApCollection>, Status> {
    if let (Signed(true, VerificationType::Local), Some(profile)) = (
        signed,
        get_profile_by_username(&conn, username.clone()).await,
    ) {
        let followers = get_followers_by_profile_id(&conn, profile.id).await;

        Ok(Json(ApCollection::from(ActorsPage {
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
        })))
    } else if let Some(profile) = get_profile_by_username(&conn, username).await {
        let followers = get_followers_by_profile_id(&conn, profile.id).await;

        Ok(Json(ApCollection::from(FollowersPage {
            page: 0,
            profile,
            followers: followers
                .iter()
                .map(|(follower, _)| follower.clone())
                .collect(),
        })))
    } else {
        Err(Status::NoContent)
    }
}

#[get("/user/<username>/following")]
pub async fn get_leaders(
    signed: Signed,
    conn: Db,
    username: String,
) -> Result<Json<ApCollection>, Status> {
    if let (Signed(true, VerificationType::Local), Some(profile)) = (
        signed,
        get_profile_by_username(&conn, username.clone()).await,
    ) {
        let leaders = get_leaders_by_profile_id(&conn, profile.id).await;

        let maybe_actors: Vec<Option<(ApActor, Option<Leader>)>> =
            join_all(leaders.iter().map(|leader| async {
                get_actor(
                    &conn,
                    leader.leader_ap_id.clone(),
                    Some(profile.clone()),
                    false,
                )
                .await
            }))
            .await;

        Ok(Json(ApCollection::from(ActorsPage {
            page: 0,
            profile,
            actors: maybe_actors
                .iter()
                .filter_map(|a| {
                    if let Some((actor, _)) = a {
                        Some(actor.clone())
                    } else {
                        None
                    }
                })
                .collect(),
        })))
    } else if let Some(profile) = get_profile_by_username(&conn, username).await {
        let leaders = get_leaders_by_profile_id(&conn, profile.id).await;

        Ok(Json(ApCollection::from(LeadersPage {
            page: 0,
            profile,
            leaders,
        })))
    } else {
        Err(Status::NoContent)
    }
}
