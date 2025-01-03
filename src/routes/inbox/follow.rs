use super::Inbox;
use crate::{
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, ActivityTarget, NewActivity,
            TryFromExtendedActivity,
        },
        actors::get_actor_by_as_id,
        followers::{create_follower, NewFollower},
    },
    runner::{self, send_to_inboxes, TaskError},
};
use jdt_activity_pub::{ApAccept, ApActivity, ApAddress, ApFollow};
use rocket::http::Status;
use serde_json::Value;

impl Inbox for ApFollow {
    async fn inbox(&self, conn: Db, raw: Value) -> Result<Status, Status> {
        let actor_as_id = self
            .object
            .clone()
            .reference()
            .ok_or(Status::UnprocessableEntity)?;

        if self.id.is_none() {
            log::error!("AP_FOLLOW ID IS NONE");
            return Err(Status::UnprocessableEntity);
        };

        let actor = get_actor_by_as_id(&conn, actor_as_id.clone())
            .await
            .map_err(|e| {
                log::error!("FAILED TO RETRIEVE ACTOR: {e:#?}");
                Status::NotFound
            })?;

        let mut activity = NewActivity::try_from((
            ApActivity::Follow(self.clone()),
            Some(ActivityTarget::from(actor)),
        ))
        .map_err(|e| {
            log::error!("FAILED TO BUILD FOLLOW ACTIVITY: {e:#?}");
            Status::InternalServerError
        })?;

        activity.raw = Some(raw);

        let activity = create_activity((&conn).into(), activity)
            .await
            .map_err(|e| {
                log::error!("FAILED TO CREATE FOLLOW ACTIVITY: {e:#?}");
                Status::InternalServerError
            })?;

        runner::run(
            process,
            conn,
            None,
            vec![activity.ap_id.ok_or(Status::InternalServerError)?],
        )
        .await;

        Ok(Status::Accepted)
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}

async fn process(
    conn: Db,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    log::debug!("PROCESSING INCOMING FOLLOW REQUEST");

    for ap_id in ap_ids {
        log::debug!("AS_ID: {ap_id}");

        let extended_follow = get_activity_by_ap_id(&conn, ap_id)
            .await
            .ok_or(TaskError::TaskFailed)?;

        let follow = ApFollow::try_from_extended_activity(extended_follow).map_err(|e| {
            log::error!("FAILED TO BUILD FOLLOW: {e:#?}");
            TaskError::TaskFailed
        })?;
        let accept = ApAccept::try_from(follow.clone()).map_err(|e| {
            log::error!("FAILED TO BUILD ACCEPT: {e:#?}");
            TaskError::TaskFailed
        })?;

        let accept_actor = get_actor_by_as_id(&conn, accept.actor.clone().to_string())
            .await
            .map_err(|e| {
                log::error!("FAILED TO RETRIEVE ACTOR: {e:#?}");
                TaskError::TaskFailed
            })?;

        let follow_actor = get_actor_by_as_id(&conn, follow.actor.clone().to_string())
            .await
            .map_err(|e| {
                log::error!("FAILED TO RETRIEVE ACTOR: {e:#?}");
                TaskError::TaskFailed
            })?;

        send_to_inboxes(
            &conn,
            vec![follow_actor.as_inbox.clone().into()],
            accept_actor.clone(),
            ApActivity::Accept(Box::new(accept)),
        )
        .await
        .map_err(|e| {
            log::error!("FAILED TO SEND ACCEPT TO INBOXES: {e:#?}");
            TaskError::TaskFailed
        })?;

        let follower = NewFollower::try_from(follow)
            .map_err(|e| {
                log::error!("FAILED TO BUILD FOLLOWER: {e:#?}");
                TaskError::TaskFailed
            })?
            .link(accept_actor.clone());

        if create_follower(Some(&conn), follower).await.is_some() {
            log::info!("FOLLOWER CREATED");
        }
    }

    Ok(())
}
