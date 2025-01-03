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
        leaders::{create_leader, NewLeader},
    },
    runner::{self, TaskError},
};
use jdt_activity_pub::{ApAccept, ApActivity, ApAddress};
use jdt_maybe_reference::MaybeReference;
use rocket::http::Status;
use serde_json::Value;

impl Inbox for Box<ApAccept> {
    #[allow(unused_variables)]
    async fn inbox(&self, conn: Db, raw: Value) -> Result<Status, Status> {
        let follow_as_id = match self.clone().object {
            MaybeReference::Reference(reference) => Some(reference),
            MaybeReference::Actual(ApActivity::Follow(actual)) => actual.id,
            _ => None,
        }
        .ok_or(Status::UnprocessableEntity)?;

        let (follow, _target_activity, _target_object, _target_actor) =
            get_activity_by_ap_id(&conn, follow_as_id)
                .await
                .ok_or(Status::NotFound)?;

        let mut accept = NewActivity::try_from((
            ApActivity::Accept(self.clone()),
            Some(follow.clone().into()),
        ))
        .map_err(|e| {
            log::error!("UNABLE TO BUILD ACCEPT ACTIVITY: {e:#?}");
            Status::InternalServerError
        })?
        .link_actor(&conn)
        .await;

        accept.link_target(Some(ActivityTarget::Activity(follow)));
        accept.raw = Some(raw);

        create_activity((&conn).into(), accept.clone())
            .await
            .map_err(|e| {
                log::error!("UNABLE TO CREATE ACCEPT ACTIVITY: {e:#?}");
                Status::InternalServerError
            })?;

        runner::run(
            process,
            conn,
            None,
            vec![accept.ap_id.clone().ok_or(Status::InternalServerError)?],
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
    as_ids: Vec<String>,
) -> Result<(), TaskError> {
    for as_id in as_ids {
        let (accept, follow, target_object, target_actor) = get_activity_by_ap_id(&conn, as_id)
            .await
            .ok_or(TaskError::TaskFailed)?;

        let accept = ApAccept::try_from_extended_activity((
            accept,
            follow.clone(),
            target_object,
            target_actor,
        ))
        .map_err(|e| {
            log::error!("ApAccept::try_from_extended_activity FAILED: {e:#?}");
            TaskError::TaskFailed
        })?;

        let follow = follow.ok_or(TaskError::TaskFailed)?;

        let profile = get_actor_by_as_id(&conn, follow.actor.to_string())
            .await
            .map_err(|e| {
                log::error!("FAILED TO RETRIEVE ACTOR: {e:#?}");
                TaskError::TaskFailed
            })?;

        let leader = NewLeader::try_from(accept.clone())
            .map_err(|e| {
                log::error!("FAILED TO BUILD LEADER: {e:#?}");
                TaskError::TaskFailed
            })?
            .link(profile);

        let leader = create_leader(Some(&conn), leader.clone())
            .await
            .ok_or(TaskError::TaskFailed)?;

        log::debug!("LEADER CREATED: {}", leader.uuid);
    }

    Ok(())
}
