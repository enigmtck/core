use crate::activity_pub::ApUndo;
use crate::routes::Outbox;

use crate::{
    activity_pub::{ApActivity, ApAddress},
    db::Db,
    fairings::events::EventChannels,
    helper::{get_local_identifier, LocalIdentifierType},
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, revoke_activity_by_uuid, ActivityType,
            NewActivity,
        },
        actors::{get_actor, Actor},
        leaders::delete_leader_by_ap_id_and_actor_id,
    },
    routes::ActivityJson,
    runner::{self, get_inboxes, send_to_inboxes, TaskError},
    MaybeReference,
};
use rocket::http::Status;
use serde_json::Value;

impl Outbox for Box<ApUndo> {
    async fn outbox(
        &self,
        conn: Db,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        ApUndo::outbox(conn, *self.clone(), profile, raw).await
    }
}

impl ApUndo {
    async fn outbox(
        conn: Db,
        undo: ApUndo,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        let target_ap_id = match undo.object {
            MaybeReference::Actual(object) => object.as_id(),
            _ => None,
        };

        let (activity, _target_activity, target_object, _target_actor) =
            get_activity_by_ap_id(&conn, target_ap_id.ok_or(Status::InternalServerError)?)
                .await
                .ok_or_else(|| {
                    log::error!("FAILED TO RETRIEVE ACTIVITY");
                    Status::NotFound
                })?;

        let mut undo = create_activity(
            Some(&conn),
            NewActivity::from((
                activity.clone(),
                ActivityType::Undo,
                ApAddress::Address(profile.as_id),
            ))
            .link_actor(&conn)
            .await,
        )
        .await
        .map_err(|e| {
            log::error!("FAILED TO CREATE ACTIVITY: {e:#?}");
            Status::InternalServerError
        })?;

        undo.raw = Some(raw);

        runner::run(
            ApUndo::send_task,
            conn,
            None,
            vec![undo.ap_id.clone().ok_or(Status::InternalServerError)?],
        )
        .await;

        let activity: ApActivity = (undo, Some(activity), target_object, None)
            .try_into()
            .map_err(|e| {
                log::error!("Failed to build ApActivity: {e:#?}");
                Status::InternalServerError
            })?;

        Ok(activity.into())
    }

    async fn send_task(
        conn: Db,
        _channels: Option<EventChannels>,
        ap_ids: Vec<String>,
    ) -> Result<(), TaskError> {
        for ap_id in ap_ids {
            let (activity, target_activity, target_object, target_actor) =
                get_activity_by_ap_id(&conn, ap_id.clone())
                    .await
                    .ok_or(TaskError::TaskFailed)?;

            let profile_id = activity.actor_id.ok_or(TaskError::TaskFailed)?;

            let sender = get_actor(&conn, profile_id)
                .await
                .ok_or(TaskError::TaskFailed)?;

            let ap_activity = ApActivity::try_from((
                activity.clone(),
                target_activity.clone(),
                target_object.clone(),
                target_actor.clone(),
            ))
            .map_err(|e| {
                log::error!("FAILED TO BUILD AP_ACTIVITY: {e:#?}");
                TaskError::TaskFailed
            })?;

            let inboxes: Vec<ApAddress> =
                get_inboxes(&conn, ap_activity.clone(), sender.clone()).await;

            send_to_inboxes(&conn, inboxes, sender, ap_activity.clone())
                .await
                .map_err(|e| {
                    log::error!("FAILED TO SEND TO INBOXES: {e:#?}");
                    TaskError::TaskFailed
                })?;

            let target_activity = ApActivity::try_from((
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
                    let id = follow.id.ok_or(TaskError::TaskFailed)?;
                    log::debug!("FOLLOW ID: {id}");
                    let identifier = get_local_identifier(id).ok_or(TaskError::TaskFailed)?;
                    log::debug!("FOLLOW IDENTIFIER: {identifier:#?}");
                    let profile_id = activity.actor_id.ok_or(TaskError::TaskFailed)?;
                    if let MaybeReference::Reference(ap_id) = follow.object {
                        if delete_leader_by_ap_id_and_actor_id(Some(&conn), ap_id, profile_id).await
                            && revoke_activity_by_uuid(Some(&conn), identifier.identifier)
                                .await
                                .is_ok()
                        {
                            log::info!("LEADER DELETED");
                        }
                    }
                }
                ApActivity::Like(like) => {
                    let id = like.id.ok_or(TaskError::TaskFailed)?;
                    log::debug!("LIKE ID: {id}");
                    let identifier = get_local_identifier(id).ok_or(TaskError::TaskFailed)?;
                    log::debug!("LIKE IDENTIFIER: {identifier:#?}");
                    if identifier.kind == LocalIdentifierType::Activity {
                        revoke_activity_by_uuid(Some(&conn), identifier.identifier)
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
                        revoke_activity_by_uuid(Some(&conn), identifier.identifier)
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
}
