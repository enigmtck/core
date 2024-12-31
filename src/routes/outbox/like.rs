use crate::activity_pub::ApLike;
use crate::routes::Outbox;

use crate::{
    activity_pub::{ApActivity, ApAddress},
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::{create_activity, get_activity_by_ap_id, NewActivity},
        actors::{get_actor, Actor},
        objects::get_object_by_as_id,
    },
    routes::ActivityJson,
    runner::{self, get_inboxes, send_to_inboxes, TaskError},
    MaybeReference,
};
use rocket::http::Status;
use serde_json::Value;

impl Outbox for Box<ApLike> {
    async fn outbox(
        &self,
        conn: Db,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        ApLike::outbox(conn, *self.clone(), profile, raw).await
    }
}

impl ApLike {
    async fn outbox(
        conn: Db,
        like: ApLike,
        _profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        if let MaybeReference::Reference(as_id) = like.clone().object {
            let object = get_object_by_as_id(Some(&conn), as_id).await.map_err(|e| {
                log::error!("FAILED TO RETRIEVE OBJECT: {e:#?}");
                Status::NotFound
            })?;

            let mut activity =
                NewActivity::try_from((Box::new(like).into(), Some(object.clone().into())))
                    .map_err(|e| {
                        log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                        Status::InternalServerError
                    })?
                    .link_actor(&conn)
                    .await;

            activity.raw = Some(raw.clone());

            let activity = create_activity(Some(&conn), activity.clone())
                .await
                .map_err(|e| {
                    log::error!("FAILED TO CREATE ACTIVITY: {e:#?}");
                    Status::InternalServerError
                })?;

            runner::run(
                ApLike::send_task,
                conn,
                None,
                vec![activity.ap_id.clone().ok_or(Status::InternalServerError)?],
            )
            .await;

            let activity: ApActivity =
                (activity, None, Some(object), None)
                    .try_into()
                    .map_err(|e| {
                        log::error!("Failed to build ApActivity: {e:#?}");
                        Status::InternalServerError
                    })?;

            Ok(activity.into())
        } else {
            log::error!("LIKE OBJECT IS NOT A REFERENCE");
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
                get_activity_by_ap_id(&conn, ap_id.clone())
                    .await
                    .ok_or(TaskError::TaskFailed)?;

            let profile_id = activity.actor_id.ok_or(TaskError::TaskFailed)?;

            let sender = get_actor(&conn, profile_id)
                .await
                .ok_or(TaskError::TaskFailed)?;

            let activity =
                ApActivity::try_from((activity, target_activity, target_object, target_actor))
                    .map_err(|e| {
                        log::error!("FAILED TO BUILD ApActivity: {e:#?}");
                        TaskError::TaskFailed
                    })?;

            let inboxes: Vec<ApAddress> =
                get_inboxes(&conn, activity.clone(), sender.clone()).await;

            send_to_inboxes(&conn, inboxes, sender, activity.clone())
                .await
                .map_err(|e| {
                    log::error!("FAILED TO SEND TO INBOXES: {e:#?}");
                    TaskError::TaskFailed
                })?;
        }

        Ok(())
    }
}
