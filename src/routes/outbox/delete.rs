use crate::routes::Outbox;
use jdt_activity_pub::{ApActivity, ApAddress, ApDelete};

use crate::routes::ActivityJson;
use crate::runner::TaskError;
use crate::{
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, NewActivity, TryFromExtendedActivity,
        },
        actors::{get_actor, Actor},
        objects::{get_object_by_as_id, tombstone_object_by_uuid},
    },
    runner,
    runner::{get_inboxes, send_to_inboxes},
};
use jdt_maybe_reference::MaybeReference;
use rocket::http::Status;
use serde_json::Value;

//impl Outbox for Box<ApDelete> {}
impl Outbox for Box<ApDelete> {
    async fn outbox(
        &self,
        conn: Db,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        delete_outbox(conn, *self.clone(), profile, raw).await
    }
}

async fn delete_outbox(
    conn: Db,
    delete: ApDelete,
    _profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, Status> {
    if let MaybeReference::Reference(as_id) = delete.clone().object {
        if delete.id.is_some() {
            return Err(Status::BadRequest);
        }

        let object = get_object_by_as_id(Some(&conn), as_id).await.map_err(|e| {
            log::error!("Target object for deletion not found: {e:#?}");
            Status::NotFound
        })?;

        let mut activity =
            NewActivity::try_from((Box::new(delete).into(), Some(object.clone().into())))
                .map_err(|e| {
                    log::error!("Failed to build Delete activity: {e:#?}");
                    Status::InternalServerError
                })?
                .link_actor(&conn)
                .await;

        activity.raw = Some(raw);

        let activity = create_activity(Some(&conn), activity.clone())
            .await
            .map_err(|e| {
                log::error!("Failed to create activity: {e:#?}");
                Status::InternalServerError
            })?;

        runner::run(
            send_task,
            conn,
            None,
            vec![activity.ap_id.clone().ok_or_else(|| {
                log::error!("Activity must have an ID");
                Status::InternalServerError
            })?],
        )
        .await;

        let activity = ApActivity::try_from_extended_activity((activity, None, Some(object), None))
            .map_err(|e| {
                log::error!("Failed to build ApActivity: {e:#?}");
                Status::InternalServerError
            })?;

        Ok(activity.into())
    } else {
        log::error!("Delete object is not a reference");
        Err(Status::NoContent)
    }
}

async fn send_task(
    conn: Db,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    for ap_id in ap_ids {
        let (activity, target_activity, target_object, target_actor) =
            get_activity_by_ap_id(&conn, ap_id).await.ok_or_else(|| {
                log::error!("Failed to retrieve activity");
                TaskError::TaskFailed
            })?;

        let profile_id = activity.actor_id.ok_or_else(|| {
            log::error!("actor_id can not be None");
            TaskError::TaskFailed
        })?;

        let sender = get_actor(&conn, profile_id).await.ok_or_else(|| {
            log::error!("Failed to retrieve Actor");
            TaskError::TaskFailed
        })?;

        let object = target_object.clone().ok_or_else(|| {
            log::error!("target_object can not be None");
            TaskError::TaskFailed
        })?;

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

        let inboxes: Vec<ApAddress> = get_inboxes(&conn, activity.clone(), sender.clone()).await;

        send_to_inboxes(&conn, inboxes, sender, activity.clone())
            .await
            .map_err(|e| {
                log::error!("Failed to send to inboxes: {e:#?}");
                TaskError::TaskFailed
            })?;

        tombstone_object_by_uuid(&conn, object.ek_uuid.ok_or(TaskError::TaskFailed)?)
            .await
            .map_err(|e| {
                log::error!("Failed to delete Objects: {e:#?}");
                TaskError::TaskFailed
            })?;
    }

    Ok(())
}
