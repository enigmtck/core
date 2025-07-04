use crate::{
    models::activities::get_unrevoked_activity_by_kind_actor_id_and_target_ap_id, routes::Outbox,
};
use jdt_activity_pub::{ApActivity, ApAddress, ApFollow};

use crate::{
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id,
            ActivityType, NewActivity, TryFromExtendedActivity,
        },
        actors::{get_actor, get_actor_by_as_id, Actor},
        follows::{create_follow, NewFollow},
    },
    routes::ActivityJson,
    runner::{self, get_inboxes, send_to_inboxes, TaskError},
};
use rocket::http::Status;
use serde_json::Value;

impl Outbox for ApFollow {
    async fn outbox(
        &self,
        conn: Db,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        follow_outbox(conn, self.clone(), profile, raw).await
    }
}

/// Handles an `ApFollow` activity in an outbox.
///
/// This function processes a follow request from a local user (`profile`) to another actor.
/// It checks if a follow activity already exists. If so, it returns the existing activity.
/// If not, it creates a new `Follow` activity, saves it to the database, and federates it.
///
/// # Arguments
///
/// * `conn` - The database connection.
/// * `follow` - The `ApFollow` activity object from the request.
/// * `profile` - The actor performing the follow action (the local user).
/// * `raw` - The raw JSON value of the request body.
///
/// # Returns
///
/// * `Ok(ActivityJson<ApActivity>)` - The created or retrieved `Follow` activity as JSON.
/// * `Err(Status)` - An HTTP status code indicating an error.
async fn follow_outbox(
    conn: Db,
    follow: ApFollow,
    profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, Status> {
    log::debug!("{follow:?}");

    // The object of a Follow activity must be a reference to the actor being followed.
    // This can be a URL string, an object with an "id" property, or a full actor object.
    // The `reference()` method correctly handles all these cases.
    let as_id = follow.object.reference().ok_or_else(|| {
        log::error!("Follow object is not a reference or does not contain an ID");
        Status::BadRequest
    })?;

    log::debug!("Follow Object: {as_id}");

    // Retrieve the actor being followed.
    let actor_to_follow = get_actor_by_as_id(Some(&conn), as_id.clone())
        .await
        .map_err(|e| {
            log::error!("Failed to retrieve actor to follow '{as_id}': {e:#?}");
            Status::NotFound
        })?;

    log::debug!("Follow Actor: {actor_to_follow}");

    // Check if a follow activity already exists. If not, create one.
    let activity = if let Some(activity) = get_unrevoked_activity_by_kind_actor_id_and_target_ap_id(
        &conn,
        ActivityType::Follow,
        profile.id,
        as_id.clone(),
    )
    .await
    {
        activity
    } else {
        // Create a new activity from the ApFollow object.
        let mut new_activity =
            NewActivity::try_from((follow.clone().into(), Some(actor_to_follow.clone().into())))
                .map_err(|e| {
                    log::error!("Failed to build NewActivity for Follow: {e:#?}");
                    Status::InternalServerError
                })?
                .link_actor(&conn)
                .await;

        new_activity.raw = Some(raw);

        // Save the new activity to the database.
        let created_activity = create_activity(Some(&conn), new_activity.clone())
            .await
            .map_err(|e| {
                log::error!("Failed to create Follow activity in DB: {e:#?}");
                Status::InternalServerError
            })?;

        // Create a corresponding Follow record.
        let mut new_follow = NewFollow::try_from(follow).map_err(|e| {
            log::error!("Failed to build NewFollow from ApFollow: {e:#?}");
            Status::InternalServerError
        })?;

        // Manually set IDs on the NewFollow record to avoid extra DB lookups.
        // - `follow_activity_ap_id` is generated by the server when creating the activity,
        //   so we get it from `created_activity`.
        // - `follower_actor_id` and `leader_actor_id` are available from `profile` and
        //   `actor_to_follow`, so we can set them directly instead of calling `link()`.
        new_follow.follow_activity_ap_id = created_activity.ap_id.clone();
        new_follow.follower_actor_id = Some(profile.id);
        new_follow.leader_actor_id = Some(actor_to_follow.id);

        create_follow(Some(&conn), new_follow).await.map_err(|e| {
            log::error!("Failed to create Follow record in DB: {e:#?}");
            Status::InternalServerError
        })?;

        created_activity
    };

    // Spawn a background task to federate the activity.
    runner::run(
        send,
        conn,
        None,
        vec![activity.ap_id.clone().ok_or_else(|| {
            log::error!("ActivityPub ID cannot be None for federation");
            Status::BadRequest
        })?],
    )
    .await;

    // Convert the database activity into an ActivityPub activity for the response.
    let ap_activity =
        ApActivity::try_from_extended_activity((activity, None, None, Some(actor_to_follow)))
            .map_err(|e| {
                log::error!("Failed to build ApActivity from ExtendedActivity: {e:#?}");
                Status::InternalServerError
            })?;

    Ok(ap_activity.into())
}

async fn send(
    conn: Db,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    for ap_id in ap_ids {
        let (activity, target_activity, target_object, target_actor) =
            get_activity_by_ap_id(&conn, ap_id.clone())
                .await
                .ok_or_else(|| {
                    log::error!("Failed to retrieve Activity");
                    TaskError::TaskFailed
                })?;

        let sender = get_actor(
            &conn,
            activity.actor_id.ok_or_else(|| {
                log::error!("Failed to retrieve Actor");
                TaskError::TaskFailed
            })?,
        )
        .await
        .ok_or(TaskError::TaskFailed)?;

        let activity = ApActivity::try_from_extended_activity((
            activity,
            target_activity,
            target_object,
            target_actor,
        ))
        .map_err(|e| {
            log::error!("Failed to build ApActivity: {e:#?}");
            TaskError::TaskFailed
        })?;

        let inboxes: Vec<ApAddress> =
            get_inboxes(Some(&conn), activity.clone(), sender.clone()).await;

        send_to_inboxes(Some(&conn), inboxes, sender, activity.clone())
            .await
            .map_err(|e| {
                log::error!("Failed to send to inboxes: {e:#?}");
                TaskError::TaskFailed
            })?;
    }
    Ok(())
}
