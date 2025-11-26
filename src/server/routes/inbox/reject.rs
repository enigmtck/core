use super::Inbox;

use crate::{
    db::runner::DbRunner,
    events::EventChannels,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, ActivityTarget, NewActivity,
            TryFromExtendedActivity,
        },
        follows::mark_follow_rejected,
    },
    runner::{self, TaskError},
    server::AppState,
};
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{ApActivity, ApAddress, ApReject};
use reqwest::StatusCode;
use serde_json::Value;

impl Inbox for Box<ApReject> {
    #[allow(unused_variables)]
    async fn inbox<C: DbRunner>(
        &self,
        conn: &C,
        state: AppState,
        raw: Value,
    ) -> Result<StatusCode, StatusCode> {
        log::info!("{self}");

        let follow_as_id = match self.clone().object {
            MaybeReference::Reference(reference) => Some(reference),
            MaybeReference::Actual(ApActivity::Follow(actual)) => actual.id,
            _ => None,
        }
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;

        let (follow, _target_activity, _target_object, _target_actor) =
            get_activity_by_ap_id(conn, follow_as_id)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?
                .ok_or_else(|| {
                    log::error!("Activity not found");
                    StatusCode::NOT_FOUND
                })?;

        let mut reject = NewActivity::try_from((
            ApActivity::Reject(self.clone()),
            Some(follow.clone().into()),
        ))
        .map_err(|e| {
            log::error!("UNABLE TO BUILD REJECT ACTIVITY: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .link_actor(conn)
        .await;

        reject.link_target(Some(ActivityTarget::Activity(follow)));
        reject.raw = Some(raw);

        create_activity(conn, reject.clone()).await.map_err(|e| {
            log::error!("UNABLE TO CREATE REJECT ACTIVITY: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        runner::run(
            process,
            state.db_pool,
            None,
            vec![reject
                .ap_id
                .clone()
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?],
        )
        .await;

        Ok(StatusCode::ACCEPTED)
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}

async fn process(
    pool: Pool,
    _channels: Option<EventChannels>,
    as_ids: Vec<String>,
) -> Result<(), TaskError> {
    for as_id in as_ids {
        let conn = pool.get().await.map_err(|_| TaskError::TaskFailed)?;
        // Get the DB records for the Reject activity and its target (the Follow activity).
        let (reject_db, follow_db, target_object, target_actor) =
            get_activity_by_ap_id(&conn, as_id.clone())
                .await
                .map_err(|_| TaskError::TaskFailed)?
                .ok_or_else(|| {
                    log::error!("Activity not found: {as_id}");
                    TaskError::TaskFailed
                })?;

        // Convert the DB records into a structured ApReject object.
        let reject = ApReject::try_from_extended_activity((
            reject_db,
            follow_db.clone(),
            target_object,
            target_actor,
        ))
        .map_err(|e| {
            log::error!("ApReject::try_from_extended_activity failed: {e}");
            TaskError::TaskFailed
        })?;

        // The `follow_db` is the database record for the Follow activity that this Reject is targeting.
        let follow_activity = follow_db.ok_or_else(|| {
            log::error!("Reject activity {as_id} has no target Follow activity");
            TaskError::TaskFailed
        })?;

        // Ensure the target activity is actually a Follow activity.
        if follow_activity.kind.is_follow() {
            // The follower is the actor of the original Follow activity.
            let follower_ap_id = follow_activity.actor;
            // The leader (who rejected) is the actor of this Reject activity.
            let leader_ap_id = reject.actor.to_string();
            let reject_ap_id = reject.id.clone().ok_or(TaskError::TaskFailed)?;

            mark_follow_rejected(&conn, follower_ap_id, leader_ap_id, reject_ap_id).await;

            log::info!("Follow rejected: {reject}");
        } else {
            log::error!(
                "Target of Reject activity {as_id} is not a Follow activity, but a {:?}",
                follow_activity.kind
            );
        }
    }

    Ok(())
}
