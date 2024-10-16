use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApFollow, ApObject, Inbox, Outbox},
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, ActivityTarget, ApActivityTarget, NewActivity,
        },
        actors::Actor,
    },
    runner, MaybeReference,
};
use anyhow::anyhow;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::activity::RecursiveActivity;

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
        //inbox::activity::accept(conn, faktory, *self.clone()).await
        let follow_apid = match self.clone().object {
            MaybeReference::Reference(reference) => Some(reference),
            MaybeReference::Actual(ApActivity::Follow(actual)) => actual.id,
            _ => None,
        };

        let follow_apid = follow_apid.ok_or(Status::new(520))?;
        let (activity, target_activity, target_object) = get_activity_by_ap_id(&conn, follow_apid)
            .await
            .ok_or(Status::new(521))?;

        let activity = NewActivity::try_from((
            ApActivity::Accept(self.clone()),
            Some(ActivityTarget::from(
                target_object.ok_or(Status::InternalServerError)?,
            )),
        ) as ApActivityTarget)
        .map_err(|_| Status::new(522))?;
        log::debug!("ACTIVITY\n{activity:#?}");
        if create_activity((&conn).into(), activity.clone())
            .await
            .is_ok()
        {
            runner::run(
                runner::follow::process_accept_task,
                Some(conn),
                Some(channels),
                vec![activity.uuid.clone()],
            )
            .await;
            Ok(Status::Accepted)
        } else {
            log::error!("FAILED TO CREATE ACTIVITY RECORD");
            Err(Status::NoContent)
        }
    }
}

impl Outbox for Box<ApAccept> {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Actor,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl TryFrom<RecursiveActivity> for ApAccept {
    type Error = anyhow::Error;

    fn try_from(
        ((activity, _target_activity, _target_object), recursive): RecursiveActivity,
    ) -> Result<Self, Self::Error> {
        let recursive = recursive.ok_or(anyhow!("RECURSIVE CANNOT BE NONE"))?;
        let recursive_activity = ApActivity::try_from((recursive.clone(), None))?;
        match recursive_activity {
            ApActivity::Follow(follow) => Ok(ApAccept {
                context: Some(ApContext::default()),
                kind: ApAcceptType::default(),
                actor: activity.actor.clone().into(),
                id: activity.ap_id.map_or(
                    Some(format!(
                        "{}/activities/{}",
                        *crate::SERVER_URL,
                        activity.uuid
                    )),
                    Some,
                ),
                object: MaybeReference::Actual(ApActivity::Follow(follow)),
            }),
            _ => {
                log::error!("FAILED TO MATCH IMPLEMENTED ACCEPT: {activity:#?}");
                Err(anyhow!("FAILED TO MATCH IMPLEMENTED ACCEPT"))
            }
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
