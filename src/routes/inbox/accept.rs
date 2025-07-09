use super::Inbox;

use crate::{
    db::runner::DbRunner,
    fairings::events::EventChannels,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, ActivityTarget, NewActivity,
            TryFromExtendedActivity,
        },
        follows::mark_follow_accepted,
    },
    runner::{self, TaskError},
};
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{ApAccept, ApActivity, ApAddress};
use reqwest::StatusCode;
use serde_json::Value;

impl Inbox for Box<ApAccept> {
    #[allow(unused_variables)]
    async fn inbox<C: DbRunner>(
        &self,
        conn: &C,
        pool: Pool,
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

        let mut accept = NewActivity::try_from((
            ApActivity::Accept(self.clone()),
            Some(follow.clone().into()),
        ))
        .map_err(|e| {
            log::error!("UNABLE TO BUILD ACCEPT ACTIVITY: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .link_actor(conn)
        .await;

        accept.link_target(Some(ActivityTarget::Activity(follow)));
        accept.raw = Some(raw);

        create_activity(conn, accept.clone()).await.map_err(|e| {
            log::error!("UNABLE TO CREATE ACCEPT ACTIVITY: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        runner::run(
            process,
            pool,
            None,
            vec![accept
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
    let conn = pool.get().await.map_err(|_| TaskError::TaskFailed)?;

    for as_id in as_ids {
        // Get the DB records for the Accept activity and its target (the Follow activity).
        // Renaming variables here for clarity: `accept_db` is the Accept activity,
        // and `follow_db` is the Follow activity being accepted.
        let (accept_db, follow_db, target_object, target_actor) =
            get_activity_by_ap_id(&conn, as_id.clone())
                .await
                .map_err(|_| TaskError::TaskFailed)?
                .ok_or_else(|| {
                    log::error!("Activity not found: {as_id}");
                    TaskError::TaskFailed
                })?;

        // Convert the DB records into a structured ApAccept object.
        let accept = ApAccept::try_from_extended_activity((
            accept_db,
            follow_db.clone(),
            target_object,
            target_actor,
        ))
        .map_err(|e| {
            log::error!("ApAccept::try_from_extended_activity failed: {e}");
            TaskError::TaskFailed
        })?;

        // The `follow_db` is the database record for the Follow activity that this Accept is targeting.
        // It's more reliable to use this record than to parse the `accept.object`.
        let follow_activity = follow_db.ok_or_else(|| {
            log::error!("Accept activity {as_id} has no target Follow activity");
            TaskError::TaskFailed
        })?;

        // Ensure the target activity is actually a Follow activity.
        if follow_activity.kind.is_follow() {
            // The follower is the actor of the original Follow activity.
            let follower_ap_id = follow_activity.actor;
            // The leader is the actor of this Accept activity.
            let leader_ap_id = accept.actor.to_string();
            let accept_ap_id = accept.id.clone().ok_or(TaskError::TaskFailed)?;

            mark_follow_accepted(&conn, follower_ap_id, leader_ap_id, accept_ap_id).await;

            log::info!("Leader established: {accept}");
        } else {
            log::error!(
                "Target of Accept activity {as_id} is not a Follow activity, but a {:?}",
                follow_activity.kind
            );
        }
    }

    Ok(())
}
