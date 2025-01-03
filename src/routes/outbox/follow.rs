use crate::routes::Outbox;
use jdt_activity_pub::{ApActivity, ApAddress, ApFollow};

use crate::{
    db::Db,
    fairings::events::EventChannels,
    helper::get_activity_ap_id_from_uuid,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, get_activity_by_kind_actor_id_and_target_ap_id,
            ActivityType, NewActivity, TryFromExtendedActivity,
        },
        actors::{get_actor, get_actor_by_as_id, Actor},
    },
    routes::ActivityJson,
    runner::{self, get_inboxes, send_to_inboxes, TaskError},
};
use jdt_maybe_reference::MaybeReference;
use rocket::http::Status;
use serde_json::Value;

impl Outbox for ApFollow {
    async fn outbox(
        &self,
        conn: Db,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        outbox(conn, self.clone(), profile, raw).await
    }
}

async fn outbox(
    conn: Db,
    follow: ApFollow,
    profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, Status> {
    if let MaybeReference::Reference(as_id) = follow.object.clone() {
        let (activity, actor) = {
            if let Some(activity) = get_activity_by_kind_actor_id_and_target_ap_id(
                &conn,
                ActivityType::Follow,
                profile.id,
                as_id.clone(),
            )
            .await
            {
                let actor = get_actor_by_as_id(&conn, as_id.clone())
                    .await
                    .map_err(|e| {
                        log::error!("Failed to retrieve Actor: {e:#?}");
                        Status::NotFound
                    })?;
                (activity, actor)
            } else {
                let actor = get_actor_by_as_id(&conn, as_id).await.map_err(|e| {
                    log::error!("Failed to retrieve Actor: {e:#?}");
                    Status::NotFound
                })?;

                let mut activity =
                    NewActivity::try_from((follow.into(), Some(actor.clone().into())))
                        .map_err(|e| {
                            log::error!("Failed to build Activity: {e:#?}");
                            Status::InternalServerError
                        })?
                        .link_actor(&conn)
                        .await;

                activity.raw = Some(raw);

                (
                    create_activity(Some(&conn), activity.clone())
                        .await
                        .map_err(|e| {
                            log::error!("Failed to create Follow activity: {e:#?}");
                            Status::InternalServerError
                        })?,
                    actor,
                )
            }
        };

        runner::run(send, conn, None, vec![activity.uuid.clone()]).await;

        let activity = ApActivity::try_from_extended_activity((activity, None, None, Some(actor)))
            .map_err(|e| {
                log::error!("Failed to build ApActivity: {e:#?}");
                Status::InternalServerError
            })?;

        Ok(activity.into())
    } else {
        log::error!("Follow object is not a reference");
        Err(Status::BadRequest)
    }
}

async fn send(
    conn: Db,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    for ap_id in ap_ids {
        let (activity, target_activity, target_object, target_actor) =
            get_activity_by_ap_id(&conn, get_activity_ap_id_from_uuid(ap_id.clone()))
                .await
                .ok_or(TaskError::TaskFailed)?;

        let sender = get_actor(&conn, activity.actor_id.ok_or(TaskError::TaskFailed)?)
            .await
            .ok_or(TaskError::TaskFailed)?;

        let activity = ApActivity::try_from_extended_activity((
            activity,
            target_activity,
            target_object,
            target_actor,
        ))
        .map_err(|e| {
            log::error!("FAILED TO BUILD AP_ACTIVITY: {e:#?}");
            TaskError::TaskFailed
        })?;

        let inboxes: Vec<ApAddress> = get_inboxes(&conn, activity.clone(), sender.clone()).await;

        send_to_inboxes(&conn, inboxes, sender, activity.clone())
            .await
            .map_err(|e| {
                log::error!("FAILED TO SEND TO INBOXES: {e:#?}");
                TaskError::TaskFailed
            })?;
    }
    Ok(())
}
