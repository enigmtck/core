use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox},
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, ActivityTarget, ExtendedActivity, NewActivity,
        },
        actors::{get_actor, Actor},
        objects::get_object_by_as_id,
    },
    routes::ActivityJson,
    runner::{self, get_inboxes, send_to_inboxes, TaskError},
    MaybeMultiple, MaybeReference,
};
use anyhow::anyhow;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApLikeType {
    #[default]
    #[serde(alias = "like")]
    Like,
}

impl fmt::Display for ApLikeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApLike {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApLikeType,
    pub actor: ApAddress,
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub to: MaybeMultiple<ApAddress>,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
}

impl Inbox for Box<ApLike> {
    async fn inbox(
        &self,
        conn: Db,
        _channels: EventChannels,
        raw: Value,
    ) -> Result<Status, Status> {
        let note_apid = match self.object.clone() {
            MaybeReference::Reference(reference) => Some(reference),
            MaybeReference::Actual(ApObject::Note(actual)) => actual.id,
            _ => None,
        };

        let note_apid = note_apid.ok_or(Status::BadRequest)?;

        let target = get_object_by_as_id(Some(&conn), note_apid)
            .await
            .map_err(|e| {
                log::debug!("LIKE TARGET NOT FOUND: {e:#?}");
                Status::NotFound
            })?;

        let mut activity = NewActivity::try_from((
            ApActivity::Like(self.clone()),
            Some(ActivityTarget::from(target)),
        ))
        .map_err(|e| {
            log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
            Status::InternalServerError
        })?;
        activity.raw = Some(raw.clone());

        create_activity((&conn).into(), activity.clone())
            .await
            .map_err(|e| {
                log::error!("FAILED TO CREATE ACTIVITY: {e:#?}");
                Status::InternalServerError
            })?;

        Ok(Status::Accepted)
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}

impl Outbox for Box<ApLike> {
    async fn outbox(
        &self,
        conn: Db,
        events: EventChannels,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        ApLike::outbox(conn, events, *self.clone(), profile, raw).await
    }
}

impl ApLike {
    async fn outbox(
        conn: Db,
        channels: EventChannels,
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
                Some(conn),
                Some(channels),
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
        conn: Option<Db>,
        _channels: Option<EventChannels>,
        ap_ids: Vec<String>,
    ) -> Result<(), TaskError> {
        let conn = conn.as_ref();

        for ap_id in ap_ids {
            let (activity, target_activity, target_object, target_actor) =
                get_activity_by_ap_id(conn.ok_or(TaskError::TaskFailed)?, ap_id.clone())
                    .await
                    .ok_or(TaskError::TaskFailed)?;

            let profile_id = activity.actor_id.ok_or(TaskError::TaskFailed)?;

            let sender = get_actor(conn.unwrap(), profile_id)
                .await
                .ok_or(TaskError::TaskFailed)?;

            let activity =
                ApActivity::try_from((activity, target_activity, target_object, target_actor))
                    .map_err(|e| {
                        log::error!("FAILED TO BUILD ApActivity: {e:#?}");
                        TaskError::TaskFailed
                    })?;

            let inboxes: Vec<ApAddress> = get_inboxes(conn, activity.clone(), sender.clone()).await;

            send_to_inboxes(conn.unwrap(), inboxes, sender, activity.clone())
                .await
                .map_err(|e| {
                    log::error!("FAILED TO SEND TO INBOXES: {e:#?}");
                    TaskError::TaskFailed
                })?;
        }

        Ok(())
    }
}

impl TryFrom<ExtendedActivity> for ApLike {
    type Error = anyhow::Error;

    fn try_from(
        (activity, _target_activity, target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if !activity.kind.is_like() {
            return Err(anyhow!("NOT A LIKE ACTIVITY"));
        }

        let object = target_object.ok_or(anyhow!("no target object"))?;
        let note = ApNote::try_from(object)?;

        let (id, object): (String, MaybeReference<ApObject>) = (
            note.attributed_to.clone().to_string(),
            MaybeReference::Reference(note.id.ok_or(anyhow!("no note id"))?),
        );

        Ok(ApLike {
            context: Some(ApContext::default()),
            kind: ApLikeType::default(),
            actor: activity.actor.into(),
            id: activity.ap_id,
            to: MaybeMultiple::Single(ApAddress::Address(id)),
            object,
        })
    }
}
