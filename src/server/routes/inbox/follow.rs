use super::Inbox;
use crate::{
    db::runner::DbRunner,
    events::EventChannels,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, ActivityTarget, NewActivity,
            TryFromExtendedActivity,
        },
        actors::get_actor_by_as_id,
        follows::{create_follow, mark_follow_accepted, NewFollow},
    },
    runner::{self, send_to_inboxes, TaskError},
};
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::{ApAccept, ApActivity, ApAddress, ApFollow};
use reqwest::StatusCode;
use serde_json::Value;

/// Handles an incoming `ApFollow` activity.
///
/// This is triggered when a remote actor wants to follow a local actor.
impl Inbox for ApFollow {
    /// 1. Validates the incoming `Follow` activity.
    /// 2. Retrieves the local actor being followed.
    /// 3. Stores the `Follow` activity in the database.
    /// 4. Schedules a background task to process the follow request, which will
    ///    either automatically accept it or mark it as pending based on the
    ///    local actor's settings.
    ///
    /// Returns `202 Accepted` to indicate that the request is being processed asynchronously.
    async fn inbox<C: DbRunner>(
        &self,
        conn: &C,
        pool: Pool,
        raw: Value,
    ) -> Result<StatusCode, StatusCode> {
        log::info!("{}", self.clone());

        let actor_as_id = self
            .object
            .clone()
            .reference()
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;

        if self.id.is_none() {
            log::error!("AP_FOLLOW ID IS NONE");
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        };

        let actor = get_actor_by_as_id(conn, actor_as_id.clone())
            .await
            .map_err(|e| {
                log::error!("FAILED TO RETRIEVE ACTOR: {e:#?}");
                StatusCode::NOT_FOUND
            })?;

        let mut activity = NewActivity::try_from((
            ApActivity::Follow(self.clone()),
            Some(ActivityTarget::from(actor)),
        ))
        .map_err(|e| {
            log::error!("FAILED TO BUILD FOLLOW ACTIVITY: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        activity.raw = Some(raw);

        let activity = create_activity(conn, activity).await.map_err(|e| {
            log::error!("FAILED TO CREATE FOLLOW ACTIVITY: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let pool = pool.clone();
        let ap_id = activity.ap_id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        tokio::spawn(async move {
            if let Err(e) = process(pool, None, vec![ap_id]).await {
                log::error!("Failed to run follow process task: {e:?}");
            }
        });

        Ok(StatusCode::ACCEPTED)
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}

/// A background task to process a `Follow` request after it has been received and stored.
///
/// This function performs the core logic of handling a follow:
/// 1. Retrieves the actors involved (the remote follower and the local leader).
/// 2. Creates a `Follow` record in the database to represent the relationship.
/// 3. Checks if the local actor requires manual approval for followers.
///    - If yes, the process stops here. The follow is pending.
///    - If no, it proceeds to automatically accept the follow.
/// 4. To accept, it creates an `Accept` activity, stores it, and sends it back to the follower's inbox.
/// 5. It then updates the `Follow` record to mark it as accepted.
async fn process(
    pool: Pool,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    log::debug!("Processing incoming follow request");
    //let conn = pool.get().await.map_err(|_| TaskError::TaskFailed)?;

    for ap_id in ap_ids {
        let conn = pool.get().await.map_err(|_| TaskError::TaskFailed)?;
        // 1. Retrieve the Follow activity and the actors involved.
        let extended_follow = get_activity_by_ap_id(&conn, ap_id.clone())
            .await
            .map_err(|_| TaskError::TaskFailed)?
            .ok_or_else(|| {
                log::error!("Activity not found: {ap_id}");
                TaskError::TaskFailed
            })?;

        let follow =
            ApFollow::try_from_extended_activity(extended_follow.clone()).map_err(|e| {
                log::error!("Failed to build ApFollow from extended activity: {e:#?}");
                TaskError::TaskFailed
            })?;

        let leader_as_id = follow.object.reference().ok_or_else(|| {
            log::error!("Follow object has no reference ID");
            TaskError::TaskFailed
        })?;

        let leader_actor = get_actor_by_as_id(&conn, leader_as_id).await.map_err(|e| {
            log::error!("Failed to retrieve leader actor: {e:#?}");
            TaskError::TaskFailed
        })?;

        let follower_actor = get_actor_by_as_id(&conn, follow.actor.clone().to_string())
            .await
            .map_err(|e| {
                log::error!("Failed to retrieve follower actor: {e:#?}");
                TaskError::TaskFailed
            })?;

        // 2. Create the Follow record in the database.
        let mut new_follow = NewFollow::try_from(follow.clone())
            .map_err(|e| {
                log::error!("Failed to build NewFollow from ApFollow: {e:#?}");
                TaskError::TaskFailed
            })?
            .link(&conn)
            .await;

        // The follow activity ID comes from the activity we just stored.
        new_follow.follow_activity_ap_id = Some(ap_id);

        if create_follow(&conn, new_follow).await.is_err() {
            // This could be a race condition where the follow already exists.
            // We can probably ignore this error and continue, as the goal is to ensure the follow exists.
            log::warn!("Failed to create Follow record, it might already exist. Continuing.");
        }

        // 3. Check if the leader requires manual approval.
        if leader_actor.ap_manually_approves_followers {
            log::info!(
                "Actor {:?} requires manual follow approval. Follow from {:?} is now pending.",
                leader_actor.as_preferred_username,
                follower_actor.as_preferred_username
            );
            return Ok(());
        }

        // 4. Automatically accept the follow by creating and sending an Accept activity.
        let mut accept = ApAccept::try_from(follow.clone()).map_err(|e| {
            log::error!("Failed to build ApAccept from ApFollow: {e:#?}");
            TaskError::TaskFailed
        })?;

        let mut accept_activity = NewActivity::try_from((
            ApActivity::Accept(Box::new(accept.clone())),
            Some(ActivityTarget::Activity(extended_follow.0.clone())),
        ))
        .map_err(|e| {
            log::error!("Failed to build NewActivity for Accept: {e:#?}");
            TaskError::TaskFailed
        })?;

        accept_activity.link_actor(&conn).await;
        accept.id = accept_activity.ap_id.clone();

        let created_accept_activity =
            create_activity(&conn, accept_activity).await.map_err(|e| {
                log::error!("Failed to create Accept activity in DB: {e:#?}");
                TaskError::TaskFailed
            })?;

        // 5. Update the Follow record to mark it as accepted.
        mark_follow_accepted(
            &conn,
            follower_actor.as_id.clone(),
            leader_actor.as_id.clone(),
            created_accept_activity.ap_id.clone().unwrap(),
        )
        .await;

        log::info!("{accept}");

        // 6. Send the Accept activity to the follower.
        send_to_inboxes(
            &conn,
            vec![follower_actor.as_inbox.clone().into()],
            leader_actor.clone(),
            ApActivity::Accept(Box::new(accept)),
        )
        .await
        .map_err(|e| {
            log::error!("Failed to send Accept to inboxes: {e:#?}");
            TaskError::TaskFailed
        })?;
    }

    Ok(())
}
