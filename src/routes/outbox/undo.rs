use crate::{
    db::runner::DbRunner,
    models::{activities::revoke_activity_by_apid, follows::delete_follow},
    routes::Outbox,
};
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::{ApActivity, ApAddress, ApUndo};
use reqwest::StatusCode;

use crate::{
    fairings::events::EventChannels,
    helper::{get_local_identifier, LocalIdentifierType},
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, revoke_activity_by_uuid, ActivityType,
            NewActivity, TryFromExtendedActivity,
        },
        actors::{get_actor, Actor},
    },
    routes::ActivityJson,
    runner::{self, get_inboxes, send_to_inboxes, TaskError},
};
use jdt_activity_pub::MaybeReference;
use serde_json::Value;

impl Outbox for Box<ApUndo> {
    async fn outbox<C: DbRunner>(
        &self,
        conn: &C,
        pool: Pool,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        undo_outbox(conn, pool, *self.clone(), profile, raw).await
    }
}

async fn undo_outbox<C: DbRunner>(
    conn: &C,
    pool: Pool,
    undo: ApUndo,
    profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, StatusCode> {
    let target_ap_id = match undo.object {
        MaybeReference::Actual(object) => object.as_id(),
        _ => None,
    };

    let (activity, _target_activity, target_object, _target_actor) =
        get_activity_by_ap_id(conn, target_ap_id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?)
            .await
            .map_err(|e| {
                log::error!("FAILED TO RETRIEVE ACTIVITY: {e}");
                StatusCode::NOT_FOUND
            })?
            .ok_or_else(|| {
                log::error!("Activity not found");
                StatusCode::NOT_FOUND
            })?;

    let mut undo = create_activity(
        conn,
        NewActivity::from((
            activity.clone(),
            ActivityType::Undo,
            ApAddress::Address(profile.as_id),
        ))
        .link_actor(conn)
        .await,
    )
    .await
    .map_err(|e| {
        log::error!("FAILED TO CREATE ACTIVITY: {e:#?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    undo.raw = Some(raw);

    runner::run(
        send_task,
        pool,
        None,
        vec![undo
            .ap_id
            .clone()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?],
    )
    .await;

    let activity =
        ApActivity::try_from_extended_activity((undo, Some(activity), target_object, None))
            .map_err(|e| {
                log::error!("Failed to build ApActivity: {e:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    Ok(ActivityJson(activity))
}

async fn send_task(
    pool: Pool,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = pool.get().await.map_err(|_| TaskError::TaskFailed)?;

    for ap_id in ap_ids {
        let (activity, target_activity, target_object, target_actor) =
            get_activity_by_ap_id(&conn, ap_id.clone())
                .await
                .map_err(|_| TaskError::TaskFailed)?
                .ok_or_else(|| {
                    log::error!("Activity not found: {ap_id}");
                    TaskError::TaskFailed
                })?;

        let profile_id = activity.actor_id.ok_or(TaskError::TaskFailed)?;

        let sender = get_actor(&conn, profile_id)
            .await
            .map_err(|_| TaskError::TaskFailed)?;

        let ap_activity = ApActivity::try_from_extended_activity((
            activity.clone(),
            target_activity.clone(),
            target_object.clone(),
            target_actor.clone(),
        ))
        .map_err(|e| {
            log::error!("FAILED TO BUILD AP_ACTIVITY: {e:#?}");
            TaskError::TaskFailed
        })?;

        let inboxes: Vec<ApAddress> = get_inboxes(&conn, ap_activity.clone(), sender.clone()).await;

        send_to_inboxes(&conn, inboxes, sender, ap_activity.clone())
            .await
            .map_err(|e| {
                log::error!("FAILED TO SEND TO INBOXES: {e:#?}");
                TaskError::TaskFailed
            })?;

        let target_activity = ApActivity::try_from_extended_activity((
            target_activity.ok_or(TaskError::TaskFailed)?,
            None,
            target_object,
            target_actor,
        ))
        .map_err(|e| {
            log::error!("FAILED TO BUILD TARGET_ACTIVITY: {e:#?}");
            TaskError::TaskFailed
        })?;

        match target_activity {
            ApActivity::Follow(follow) => {
                let follow_activity_ap_id = follow.id.ok_or(TaskError::TaskFailed)?;
                log::debug!("ApFollow ID to Undo: {follow_activity_ap_id}");

                let leader_ap_id = follow.object.reference().ok_or(TaskError::TaskFailed)?;
                if delete_follow(&conn, follow.actor.to_string(), leader_ap_id)
                    .await
                    .is_ok()
                    && revoke_activity_by_apid(&conn, follow_activity_ap_id)
                        .await
                        .is_ok()
                {
                    log::info!("Leader deleted");
                }
            }
            ApActivity::Like(like) => {
                let id = like.id.ok_or(TaskError::TaskFailed)?;
                log::debug!("LIKE ID: {id}");
                let identifier = get_local_identifier(id).ok_or(TaskError::TaskFailed)?;
                log::debug!("LIKE IDENTIFIER: {identifier:#?}");
                if identifier.kind == LocalIdentifierType::Activity {
                    revoke_activity_by_uuid(&conn, identifier.identifier)
                        .await
                        .map_err(|e| {
                            log::error!("LIKE REVOCATION FAILED: {e:#?}");
                            TaskError::TaskFailed
                        })?;
                };
            }

            ApActivity::Announce(announce) => {
                let id = announce.id.ok_or(TaskError::TaskFailed)?;
                log::debug!("ANNOUNCE ID: {id}");
                let identifier = get_local_identifier(id).ok_or(TaskError::TaskFailed)?;
                log::debug!("ANNOUNCE IDENTIFIER: {identifier:#?}");
                if identifier.kind == LocalIdentifierType::Activity {
                    revoke_activity_by_uuid(&conn, identifier.identifier)
                        .await
                        .map_err(|e| {
                            log::error!("ANNOUNCE REVOCATION FAILED: {e:#?}");
                            TaskError::TaskFailed
                        })?;
                }
            }
            _ => {
                log::error!("FAILED TO MATCH REVOCABLE ACTIVITY");
                return Err(TaskError::TaskFailed);
            }
        }
    }

    Ok(())
}
