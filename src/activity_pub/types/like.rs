use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox},
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::get_activity_by_ap_id,
        activities::{create_activity, ActivityTarget, ExtendedActivity, NewActivity},
        actors::get_actor,
        actors::Actor,
        objects::get_object_by_as_id,
    },
    runner,
    runner::{get_inboxes, send_to_inboxes, TaskError},
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
    #[serde(skip_serializing)]
    pub to: Option<MaybeMultiple<ApAddress>>,
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

        log::debug!("NOTE AP_ID\n{note_apid:#?}");

        let target = get_object_by_as_id(Some(&conn), note_apid)
            .await
            .map_err(|_| Status::NotFound)?;

        log::debug!("TARGET\n{target:#?}");

        let mut activity = NewActivity::try_from((
            ApActivity::Like(self.clone()),
            Some(ActivityTarget::from(target)),
        ))
        .map_err(|_| Status::InternalServerError)?;
        activity.raw = Some(raw.clone());

        log::debug!("ACTIVITY\n{activity:#?}");

        create_activity((&conn).into(), activity.clone())
            .await
            .map_err(|_| Status::InternalServerError)?;

        Ok(Status::Accepted)
    }
}

impl Outbox for Box<ApLike> {
    async fn outbox(
        &self,
        conn: Db,
        events: EventChannels,
        profile: Actor,
        raw: Value,
    ) -> Result<String, Status> {
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
    ) -> Result<String, Status> {
        if let MaybeReference::Reference(as_id) = like.clone().object {
            let object = get_object_by_as_id(Some(&conn), as_id).await.map_err(|e| {
                log::error!("FAILED TO RETRIEVE OBJECT: {e:#?}");
                Status::NotFound
            })?;

            let mut activity = NewActivity::try_from((Box::new(like).into(), Some(object.into())))
                .map_err(|e| {
                    log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                    Status::InternalServerError
                })?
                .link_actor(&conn)
                .await;

            activity.raw = Some(raw.clone());

            create_activity(Some(&conn), activity.clone())
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
            Ok(activity.ap_id.clone().ok_or(Status::InternalServerError)?)
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
            log::debug!("LOOKING FOR AP_ID {ap_id}");

            let (activity, target_activity, target_object, target_actor) =
                get_activity_by_ap_id(conn.ok_or(TaskError::TaskFailed)?, ap_id.clone())
                    .await
                    .ok_or(TaskError::TaskFailed)?;

            log::debug!("FOUND ACTIVITY\n{activity:#?}");
            let profile_id = activity.actor_id.ok_or(TaskError::TaskFailed)?;

            let sender = get_actor(conn.unwrap(), profile_id)
                .await
                .ok_or(TaskError::TaskFailed)?;

            let activity =
                ApActivity::try_from((activity, target_activity, target_object, target_actor))
                    .map_err(|_| TaskError::TaskFailed)?;

            let inboxes: Vec<ApAddress> = get_inboxes(conn, activity.clone(), sender.clone()).await;

            log::debug!("SENDING LIKE\n{inboxes:#?}\n{sender:#?}\n{activity:#?}");
            send_to_inboxes(conn.unwrap(), inboxes, sender, activity.clone())
                .await
                .map_err(|_| TaskError::TaskFailed)?;
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
            to: Some(MaybeMultiple::Single(ApAddress::Address(id))),
            object,
        })
    }
}
