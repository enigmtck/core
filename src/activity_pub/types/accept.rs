use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApFollow, ApObject, Inbox, Outbox},
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, ActivityTarget, ExtendedActivity, NewActivity,
        },
        actors::{get_actor_by_as_id, Actor},
        leaders::{create_leader, NewLeader},
    },
    runner::{self, TaskError},
    MaybeReference,
};
use anyhow::anyhow;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApAcceptType {
    #[default]
    #[serde(alias = "accept")]
    Accept,
}

impl fmt::Display for ApAcceptType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApAccept {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApAcceptType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub object: MaybeReference<ApActivity>,
}

impl Inbox for Box<ApAccept> {
    #[allow(unused_variables)]
    async fn inbox(&self, conn: Db, channels: EventChannels, raw: Value) -> Result<Status, Status> {
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
            ApAccept::process,
            Some(conn),
            Some(channels),
            vec![accept.ap_id.clone().ok_or(Status::InternalServerError)?],
        )
        .await;

        Ok(Status::Accepted)
    }
}

impl ApAccept {
    async fn process(
        conn: Option<Db>,
        _channels: Option<EventChannels>,
        as_ids: Vec<String>,
    ) -> Result<(), TaskError> {
        let conn = conn.as_ref();

        for as_id in as_ids {
            let (accept, follow, target_object, target_actor) =
                get_activity_by_ap_id(conn.ok_or(TaskError::TaskFailed)?, as_id)
                    .await
                    .ok_or(TaskError::TaskFailed)?;

            let accept = ApAccept::try_from((accept, follow.clone(), target_object, target_actor))
                .map_err(|e| {
                    log::error!("ApAccept::try_from FAILED: {e:#?}");
                    TaskError::TaskFailed
                })?;

            let follow = follow.ok_or(TaskError::TaskFailed)?;

            let profile = get_actor_by_as_id(conn.unwrap(), follow.actor.to_string())
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

            let leader = create_leader(conn, leader.clone())
                .await
                .ok_or(TaskError::TaskFailed)?;

            log::debug!("LEADER CREATED: {}", leader.uuid);
        }

        Ok(())
    }
}

impl Outbox for Box<ApAccept> {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Actor,
        _raw: Value,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl TryFrom<ExtendedActivity> for ApAccept {
    type Error = anyhow::Error;

    fn try_from(
        (activity, target_activity, _target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        let follow = ApActivity::try_from((
            target_activity.ok_or(anyhow!("TARGET_ACTIVITY CANNOT BE NONE"))?,
            None,
            None,
            None,
        ))?;

        if let ApActivity::Follow(follow) = follow {
            Ok(ApAccept {
                context: Some(ApContext::default()),
                kind: ApAcceptType::default(),
                actor: activity.actor.clone().into(),
                id: Some(activity.ap_id.ok_or(anyhow!("ACCEPT MUST HAVE AN AP_ID"))?),
                object: MaybeReference::Actual(ApActivity::Follow(follow)),
            })
        } else {
            log::error!(
                "FAILED TO MATCH IMPLEMENTED ACCEPT IN TryFrom FOR ApAccept\n{activity:#?}"
            );
            Err(anyhow!("FAILED TO MATCH IMPLEMENTED ACCEPT"))
        }
    }
}

impl TryFrom<ApFollow> for ApAccept {
    type Error = anyhow::Error;

    fn try_from(follow: ApFollow) -> Result<Self, Self::Error> {
        let actor = {
            match follow.object.clone() {
                MaybeReference::Actual(ApObject::Actor(actual)) => actual.id,
                MaybeReference::Reference(reference) => Some(ApAddress::Address(reference)),
                _ => None,
            }
        };

        if let Some(actor) = actor {
            Ok(ApAccept {
                context: Some(ApContext::default()),
                kind: ApAcceptType::default(),
                actor,
                id: follow.id.clone().map(|id| format!("{id}#accept")),
                object: MaybeReference::Actual(ApActivity::Follow(follow)),
            })
        } else {
            Err(anyhow!("COULD NOT IDENTIFY ACTOR"))
        }
    }
}
