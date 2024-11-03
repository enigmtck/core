use crate::{
    activity_pub::{ApActor, ApCollection, ApCollectionAmbiguated, FollowersPage, LeadersPage},
    db::Db,
    fairings::signatures::Signed,
    models::{
        actors::get_actor_by_username, followers::get_follower_count_by_actor_id,
        followers::get_followers_by_actor_id, leaders::get_leader_count_by_actor_id,
        leaders::get_leaders_by_actor_id, OffsetPaging,
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

#[get("/user/<username>/followers?<page>")]
pub async fn get_followers(
    _signed: Signed,
    conn: Db,
    username: String,
    page: Option<u32>,
) -> Result<ActivityJson<ApCollectionAmbiguated>, Status> {
    let profile = get_actor_by_username(&conn, username)
        .await
        .ok_or(Status::NotFound)?;

    let total_items = get_follower_count_by_actor_id(&conn, profile.id)
        .await
        .map_err(|e| {
            log::error!("FAILED TO RETRIEVE FOLLOWER COUNT: {e:#?}");
            Status::InternalServerError
        })?;

    let results = match page {
        Some(p) if p > 0 => {
            get_followers_by_actor_id(
                &conn,
                profile.id,
                Some(OffsetPaging {
                    page: p - 1,
                    limit: 20,
                }),
            )
            .await
        }
        _ => vec![],
    };

    let followers = results
        .iter()
        .map(|(follower, _)| follower.clone())
        .collect();

    let actors = Some(
        results
            .iter()
            .filter_map(|(_, actor)| actor.clone())
            .collect::<Vec<_>>(),
    );

    Ok(ActivityJson(Json(
        ApCollectionAmbiguated::try_from(FollowersPage {
            page,
            profile,
            total_items,
            followers,
            actors,
        })
        .map_err(|e| {
            log::error!("FAILED TO RETRIEVE LEADERS: {e:#?}");
            Status::InternalServerError
        })?,
    )))
}

#[get("/user/<username>/following?<page>")]
pub async fn get_leaders(
    _signed: Signed,
    conn: Db,
    username: String,
    page: Option<u32>, // page starts at 1; must be adjusted to 0 for query
) -> Result<ActivityJson<ApCollectionAmbiguated>, Status> {
    let profile = get_actor_by_username(&conn, username)
        .await
        .ok_or(Status::NotFound)?;

    let total_items = get_leader_count_by_actor_id(&conn, profile.id)
        .await
        .map_err(|e| {
            log::error!("FAILED TO RETRIEVE LEADER COUNT: {e:#?}");
            Status::InternalServerError
        })?;

    let results = match page {
        Some(p) if p > 0 => {
            get_leaders_by_actor_id(
                &conn,
                profile.id,
                Some(OffsetPaging {
                    page: p - 1,
                    limit: 20,
                }),
            )
            .await
        }
        _ => vec![],
    };

    let leaders = results.iter().map(|(leader, _)| leader.clone()).collect();
    let actors = Some(
        results
            .iter()
            .filter_map(|(_, actor)| actor.clone())
            .collect::<Vec<_>>(),
    );

    Ok(ActivityJson(Json(
        ApCollectionAmbiguated::try_from(LeadersPage {
            page,
            profile,
            total_items,
            leaders,
            actors,
        })
        .map_err(|e| {
            log::error!("FAILED TO RETRIEVE LEADERS: {e:#?}");
            Status::InternalServerError
        })?,
    )))
}
