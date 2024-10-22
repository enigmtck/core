use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApObject, Inbox, Outbox},
    db::Db,
    fairings::events::EventChannels,
    helper::get_activity_ap_id_from_uuid,
    models::{
        activities::{create_activity, ActivityTarget, ExtendedActivity, NewActivity},
        actors::{get_actor_by_as_id, Actor},
        from_serde,
    },
    runner, MaybeMultiple, MaybeReference,
};
use anyhow::anyhow;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApFollowType {
    #[default]
    #[serde(alias = "follow")]
    Follow,
}

impl fmt::Display for ApFollowType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApFollow {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApFollowType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub to: Option<MaybeMultiple<ApAddress>>,
    pub cc: Option<MaybeMultiple<ApAddress>>,
    pub object: MaybeReference<ApObject>,
}

impl Inbox for ApFollow {
    async fn inbox(&self, conn: Db, channels: EventChannels, raw: Value) -> Result<Status, Status> {
        let actor_as_id = self
            .object
            .clone()
            .reference()
            .ok_or(Status::UnprocessableEntity)?;

        if self.id.is_none() {
            return Err(Status::UnprocessableEntity);
        };

        let actor = get_actor_by_as_id(&conn, actor_as_id.clone())
            .await
            .ok_or(Status::NotFound)?;

        let mut activity = NewActivity::try_from((
            ApActivity::Follow(self.clone()),
            Some(ActivityTarget::from(actor)),
        ))
        .map_err(|_| Status::InternalServerError)?;

        activity.raw = Some(raw);

        log::debug!("ACTIVITY\n{activity:#?}");

        let activity = create_activity((&conn).into(), activity)
            .await
            .map_err(|_| Status::InternalServerError)?;

        runner::run(
            runner::follow::acknowledge_followers_task,
            Some(conn),
            Some(channels),
            vec![activity.ap_id.ok_or(Status::InternalServerError)?],
        )
        .await;
        Ok(Status::Accepted)
    }
}

impl Outbox for ApFollow {
    async fn outbox(
        &self,
        conn: Db,
        events: EventChannels,
        profile: Actor,
        raw: Value,
    ) -> Result<String, Status> {
        outbox(conn, events, self.clone(), profile).await
    }
}

async fn outbox(
    conn: Db,
    channels: EventChannels,
    follow: ApFollow,
    profile: Actor,
) -> Result<String, Status> {
    if let MaybeReference::Reference(id) = follow.object.clone() {
        let actor = get_actor_by_as_id(&conn, id)
            .await
            .ok_or(Status::NotFound)?;

        if let Ok(activity) = create_activity(
            Some(&conn),
            NewActivity::try_from((follow.into(), Some(actor.into())))
                .map_err(|_| Status::InternalServerError)?
                .link_actor(&conn)
                .await,
        )
        .await
        {
            runner::run(
                runner::follow::process_follow_task,
                Some(conn),
                Some(channels),
                vec![activity.uuid.clone()],
            )
            .await;
            Ok(get_activity_ap_id_from_uuid(activity.uuid))
        } else {
            log::error!("FAILED TO CREATE FOLLOW ACTIVITY");
            Err(Status::NoContent)
        }
    } else {
        log::error!("FOLLOW OBJECT IS NOT A REFERENCE");
        Err(Status::NoContent)
    }
}

impl TryFrom<ExtendedActivity> for ApFollow {
    type Error = anyhow::Error;

    fn try_from(
        (activity, _target_activity, _target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if activity.kind.is_follow() {
            let target = activity
                .target_ap_id
                .ok_or(anyhow!("no target_ap_id on follow"))?;
            Ok(ApFollow {
                context: Some(ApContext::default()),
                kind: ApFollowType::default(),
                actor: activity.actor.into(),
                id: Some(activity.ap_id.ok_or(anyhow!("no follow as_id found"))?),
                to: activity.ap_to.and_then(from_serde),
                cc: activity.cc.and_then(from_serde),
                object: target.into(),
            })
        } else {
            log::error!("NOT A FOLLOW ACTIVITY");
            Err(anyhow!("NOT A FOLLOW ACTIVITY"))
        }
    }
}
