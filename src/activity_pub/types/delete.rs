use core::fmt;
use std::fmt::Debug;

use crate::runner::TaskError;
use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox},
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::get_activity_by_ap_id,
        activities::{
            create_activity, revoke_activities_by_object_as_id, ActivityTarget, NewActivity,
        },
        actors::{get_actor, get_actor_by_as_id, tombstone_actor_by_as_id, Actor},
        objects::{get_object_by_as_id, tombstone_object_by_as_id, tombstone_object_by_uuid},
        Tombstone,
    },
    runner,
    runner::{
        //encrypted::handle_encrypted_note,
        get_inboxes,
        send_to_inboxes,
    },
    MaybeMultiple, MaybeReference,
};
use anyhow::anyhow;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::signature::ApSignature;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApDeleteType {
    #[default]
    #[serde(alias = "delete")]
    Delete,
}

impl fmt::Display for ApDeleteType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApDelete {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApDeleteType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
    pub signature: Option<ApSignature>,
    pub to: MaybeMultiple<ApAddress>,
    pub cc: Option<MaybeMultiple<ApAddress>>,
}

impl Inbox for Box<ApDelete> {
    async fn inbox(
        &self,
        conn: Db,
        _channels: EventChannels,
        raw: Value,
    ) -> Result<Status, Status> {
        let tombstone = match self.object.clone() {
            MaybeReference::Actual(actual) => match actual {
                ApObject::Tombstone(tombstone) => Ok(async {
                    match get_actor_by_as_id(&conn, tombstone.id.clone()).await.ok() {
                        Some(actor) => Some(Tombstone::Actor(actor)),
                        None => get_object_by_as_id(Some(&conn), tombstone.id.clone())
                            .await
                            .ok()
                            .map(Tombstone::Object),
                    }
                }
                .await
                .ok_or_else(|| {
                    log::error!("Failed to identify Tombstone: {}", tombstone.id);
                    Status::NotFound
                })?),
                ApObject::Identifier(obj) => Ok(async {
                    match get_actor_by_as_id(&conn, obj.id.clone()).await.ok() {
                        Some(actor) => Some(Tombstone::Actor(actor)),
                        None => get_object_by_as_id(Some(&conn), obj.id.clone())
                            .await
                            .ok()
                            .map(Tombstone::Object),
                    }
                }
                .await
                .ok_or_else(|| {
                    log::error!("Failed to determine Identifier: {}", obj.id);
                    Status::NotFound
                })?),
                _ => {
                    log::debug!("Failed to identify Delete Object: {:#?}", self.object);
                    Err(Status::NoContent)
                }
            },
            MaybeReference::Reference(ap_id) => Ok(async {
                match get_actor_by_as_id(&conn, ap_id.clone()).await.ok() {
                    Some(actor) => Some(Tombstone::Actor(actor)),
                    None => get_object_by_as_id(Some(&conn), ap_id.clone())
                        .await
                        .ok()
                        .map(Tombstone::Object),
                }
            }
            .await
            .ok_or_else(|| {
                log::error!("Failed to identify Tombstone");
                Status::NotFound
            })?),
            _ => Err(Status::NotImplemented),
        };

        let tombstone = tombstone.clone()?;

        let mut activity = match tombstone.clone() {
            Tombstone::Actor(actor) => NewActivity::try_from((
                ApActivity::Delete(self.clone()),
                Some(ActivityTarget::from(actor.clone())),
            ))
            .map_err(|e| {
                log::error!("Failed to build Activity: {e:#?}");
                Status::InternalServerError
            })?,
            Tombstone::Object(object) => NewActivity::try_from((
                ApActivity::Delete(self.clone()),
                Some(ActivityTarget::from(object.clone())),
            ))
            .map_err(|e| {
                log::error!("Failed to build Activity: {e:#?}");
                Status::InternalServerError
            })?,
        };

        activity.raw = Some(raw);

        create_activity(Some(&conn), activity).await.map_err(|e| {
            log::error!("Failed to create Activity: {e:#?}");
            Status::InternalServerError
        })?;

        match tombstone {
            Tombstone::Actor(actor) => {
                if self.actor.to_string() == actor.as_id {
                    tombstone_actor_by_as_id(&conn, actor.as_id)
                        .await
                        .map_err(|e| {
                            log::error!("Failed to delete Actor: {e:#?}");
                            Status::InternalServerError
                        })?;
                }
            }
            Tombstone::Object(object) => {
                if let Some(attributed_to) = object.attributed_to().first() {
                    if self.actor.to_string() == attributed_to.clone() {
                        tombstone_object_by_as_id(&conn, object.as_id.clone())
                            .await
                            .map_err(|e| {
                                log::error!("Failed to delete Object: {e:#?}");
                                Status::InternalServerError
                            })?;

                        revoke_activities_by_object_as_id(&conn, object.as_id)
                            .await
                            .map_err(|e| {
                                log::error!("Failed to revoke Activities: {e:#?}");
                                Status::InternalServerError
                            })?;
                    }
                }
            }
        }

        Ok(Status::Accepted)
    }
}

impl Outbox for Box<ApDelete> {
    async fn outbox(
        &self,
        conn: Db,
        events: EventChannels,
        profile: Actor,
        raw: Value,
    ) -> Result<String, Status> {
        ApDelete::outbox(conn, events, *self.clone(), profile, raw).await
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApTombstoneType {
    #[default]
    Tombstone,
}

impl fmt::Display for ApTombstoneType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApTombstone {
    #[serde(rename = "type")]
    pub kind: ApTombstoneType,
    pub id: String,
    pub atom_uri: Option<String>,
}

impl Outbox for ApTombstone {
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

impl TryFrom<ApNote> for ApTombstone {
    type Error = anyhow::Error;

    fn try_from(note: ApNote) -> Result<Self, Self::Error> {
        if let Some(id) = note.id {
            Ok(ApTombstone {
                kind: ApTombstoneType::Tombstone,
                id: id.clone(),
                atom_uri: Some(id),
            })
        } else {
            Err(anyhow!("ApNote must have an ID"))
        }
    }
}

impl TryFrom<ApNote> for ApDelete {
    type Error = anyhow::Error;

    fn try_from(note: ApNote) -> Result<Self, Self::Error> {
        note.id.clone().ok_or(anyhow!("ApNote must have an ID"))?;
        let tombstone = ApTombstone::try_from(note.clone())?;
        Ok(ApDelete {
            context: Some(ApContext::default()),
            actor: note.attributed_to.clone(),
            kind: ApDeleteType::Delete,
            id: None, // This will be set in NewActivity
            object: MaybeReference::Actual(ApObject::Tombstone(tombstone)),
            signature: None,
            to: note.to,
            cc: note.cc,
        })
    }
}

impl ApDelete {
    async fn outbox(
        conn: Db,
        channels: EventChannels,
        delete: ApDelete,
        _profile: Actor,
        raw: Value,
    ) -> Result<String, Status> {
        if let MaybeReference::Reference(as_id) = delete.clone().object {
            if delete.id.is_some() {
                return Err(Status::BadRequest);
            }

            let object = get_object_by_as_id(Some(&conn), as_id).await.map_err(|e| {
                log::error!("Target object for deletion not found: {e:#?}");
                Status::NotFound
            })?;

            let mut activity =
                NewActivity::try_from((Box::new(delete).into(), Some(object.into())))
                    .map_err(|e| {
                        log::error!("Failed to build Delete activity: {e:#?}");
                        Status::InternalServerError
                    })?
                    .link_actor(&conn)
                    .await;

            activity.raw = Some(raw);

            create_activity(Some(&conn), activity.clone())
                .await
                .map_err(|e| {
                    log::error!("Failed to create activity: {e:#?}");
                    Status::InternalServerError
                })?;

            runner::run(
                ApDelete::send_task,
                Some(conn),
                Some(channels),
                vec![activity.ap_id.clone().ok_or_else(|| {
                    log::error!("Activity must have an ID");
                    Status::InternalServerError
                })?],
            )
            .await;
            Ok(activity.ap_id.unwrap())
        } else {
            log::error!("Delete object is not a reference");
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
                get_activity_by_ap_id(conn.ok_or(TaskError::TaskFailed)?, ap_id)
                    .await
                    .ok_or_else(|| {
                        log::error!("Failed to retrieve activity");
                        TaskError::TaskFailed
                    })?;

            let profile_id = activity.actor_id.ok_or_else(|| {
                log::error!("actor_id can not be None");
                TaskError::TaskFailed
            })?;

            let sender = get_actor(conn.unwrap(), profile_id).await.ok_or_else(|| {
                log::error!("Failed to retrieve Actor");
                TaskError::TaskFailed
            })?;

            let object = target_object.clone().ok_or_else(|| {
                log::error!("target_object can not be None");
                TaskError::TaskFailed
            })?;

            let activity =
                ApActivity::try_from((activity, target_activity, target_object, target_actor))
                    .map_err(|e| {
                        log::error!("Failed to build ApActivity: {e:#?}");
                        TaskError::TaskFailed
                    })?;

            let inboxes: Vec<ApAddress> = get_inboxes(conn, activity.clone(), sender.clone()).await;

            send_to_inboxes(conn.unwrap(), inboxes, sender, activity.clone())
                .await
                .map_err(|e| {
                    log::error!("Failed to send to inboxes: {e:#?}");
                    TaskError::TaskFailed
                })?;

            tombstone_object_by_uuid(conn.unwrap(), object.ek_uuid.ok_or(TaskError::TaskFailed)?)
                .await
                .map_err(|e| {
                    log::error!("Failed to delete Objects: {e:#?}");
                    TaskError::TaskFailed
                })?;
        }

        Ok(())
    }
}
