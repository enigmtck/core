use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApFollow, Inbox, Outbox},
    db::Db,
    fairings::events::EventChannels,
    helper::{get_local_identifier, LocalIdentifierType},
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, revoke_activity_by_apid,
            revoke_activity_by_uuid, ActivityTarget, ActivityType, ExtendedActivity, NewActivity,
        },
        actors::{get_actor, Actor},
        coalesced_activity::CoalescedActivity,
        followers::delete_follower_by_ap_id,
        leaders::delete_leader_by_ap_id_and_actor_id,
    },
    routes::ActivityJson,
    runner::{self, get_inboxes, send_to_inboxes, TaskError},
    MaybeReference,
};
use anyhow::anyhow;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApUndoType {
    #[default]
    #[serde(alias = "undo")]
    Undo,
}

impl fmt::Display for ApUndoType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApUndo {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApUndoType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub object: MaybeReference<ApActivity>,
}

impl Inbox for Box<ApUndo> {
    async fn inbox(&self, conn: Db, channels: EventChannels, raw: Value) -> Result<Status, Status> {
        match self.object.clone() {
            MaybeReference::Actual(actual) => {
                ApUndo::inbox(conn, channels, &actual, self, raw).await
            }
            MaybeReference::Reference(_) => {
                log::warn!(
                    "INSUFFICIENT CONTEXT FOR UNDO TARGET (REFERENCE FOUND WHEN ACTUAL IS REQUIRED)"
                );
                log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                Err(Status::BadRequest)
            }
            _ => {
                log::warn!(
                    "INSUFFICIENT CONTEXT FOR UNDO TARGET (NONE FOUND WHEN ACTUAL IS REQUIRED)"
                );
                log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                Err(Status::BadRequest)
            }
        }
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}

impl Outbox for Box<ApUndo> {
    async fn outbox(
        &self,
        conn: Db,
        events: EventChannels,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        ApUndo::outbox(conn, events, *self.clone(), profile, raw).await
    }
}

impl ApUndo {
    async fn inbox(
        conn: Db,
        channels: EventChannels,
        target: &ApActivity,
        undo: &ApUndo,
        raw: Value,
    ) -> Result<Status, Status> {
        let target_ap_id = target.as_id().ok_or_else(|| {
            log::error!("ApActivity.as_id() FAILED\n{target:#?}");
            Status::NotImplemented
        })?;

        let (_, target_activity, _, _) = get_activity_by_ap_id(&conn, target_ap_id.clone())
            .await
            .ok_or({
            log::error!("ACTIVITY NOT FOUND: {target_ap_id}");
            Status::NotFound
        })?;

        let activity_target = (
            ApActivity::Undo(Box::new(undo.clone())),
            Some(ActivityTarget::from(
                target_activity.ok_or(Status::NotFound)?,
            )),
        );

        let mut activity = NewActivity::try_from(activity_target).map_err(|e| {
            log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
            Status::InternalServerError
        })?;

        activity.raw = Some(raw.clone());

        create_activity(Some(&conn), activity.clone())
            .await
            .map_err(|e| {
                log::error!("FAILED TO CREATE ACTIVITY: {e:#?}");
                Status::InternalServerError
            })?;

        match target {
            ApActivity::Like(_) => {
                revoke_activity_by_apid(Some(&conn), target_ap_id.clone())
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO REVOKE LIKE: {e:#?}");
                        Status::InternalServerError
                    })?;
                Ok(Status::Accepted)
            }
            ApActivity::Follow(_) => {
                if delete_follower_by_ap_id(Some(&conn), target_ap_id.clone()).await {
                    log::info!("FOLLOWER RECORD DELETED: {target_ap_id}");
                }

                Ok(Status::Accepted)
            }
            ApActivity::Announce(_) => {
                runner::run(
                    runner::announce::remote_undo_announce_task,
                    conn,
                    Some(channels),
                    vec![target_ap_id.clone()],
                )
                .await;
                Ok(Status::Accepted)
            }
            _ => Err(Status::NotImplemented),
        }
    }

    async fn outbox(
        conn: Db,
        channels: EventChannels,
        undo: ApUndo,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        let target_ap_id = match undo.object {
            MaybeReference::Actual(object) => object.as_id(),
            _ => None,
        };

        let (activity, _target_activity, target_object, _target_actor) =
            get_activity_by_ap_id(&conn, target_ap_id.ok_or(Status::InternalServerError)?)
                .await
                .ok_or_else(|| {
                    log::error!("FAILED TO RETRIEVE ACTIVITY");
                    Status::NotFound
                })?;

        let mut undo = create_activity(
            Some(&conn),
            NewActivity::from((
                activity.clone(),
                ActivityType::Undo,
                ApAddress::Address(profile.as_id),
            ))
            .link_actor(&conn)
            .await,
        )
        .await
        .map_err(|e| {
            log::error!("FAILED TO CREATE ACTIVITY: {e:#?}");
            Status::InternalServerError
        })?;

        undo.raw = Some(raw);

        runner::run(
            ApUndo::send_task,
            conn,
            Some(channels),
            vec![undo.ap_id.clone().ok_or(Status::InternalServerError)?],
        )
        .await;

        let activity: ApActivity = (undo, Some(activity), target_object, None)
            .try_into()
            .map_err(|e| {
                log::error!("Failed to build ApActivity: {e:#?}");
                Status::InternalServerError
            })?;

        Ok(activity.into())
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

            let ap_activity = ApActivity::try_from((
                activity.clone(),
                target_activity.clone(),
                target_object.clone(),
                target_actor.clone(),
            ))
            .map_err(|e| {
                log::error!("FAILED TO BUILD AP_ACTIVITY: {e:#?}");
                TaskError::TaskFailed
            })?;

            let inboxes: Vec<ApAddress> =
                get_inboxes(&conn, ap_activity.clone(), sender.clone()).await;

            send_to_inboxes(&conn, inboxes, sender, ap_activity.clone())
                .await
                .map_err(|e| {
                    log::error!("FAILED TO SEND TO INBOXES: {e:#?}");
                    TaskError::TaskFailed
                })?;

            let target_activity = ApActivity::try_from((
                target_activity.ok_or(TaskError::TaskFailed)?,
                None,
                target_object,
                target_actor,
            ))
            .map_err(|e| {
                log::error!("FAILED TO BUILD TARGET_ACTIVITY: {e:#?}");
                TaskError::TaskFailed
            })?;

            match target_activity {
                ApActivity::Follow(follow) => {
                    let id = follow.id.ok_or(TaskError::TaskFailed)?;
                    log::debug!("FOLLOW ID: {id}");
                    let identifier = get_local_identifier(id).ok_or(TaskError::TaskFailed)?;
                    log::debug!("FOLLOW IDENTIFIER: {identifier:#?}");
                    let profile_id = activity.actor_id.ok_or(TaskError::TaskFailed)?;
                    if let MaybeReference::Reference(ap_id) = follow.object {
                        if delete_leader_by_ap_id_and_actor_id(Some(&conn), ap_id, profile_id).await
                            && revoke_activity_by_uuid(Some(&conn), identifier.identifier)
                                .await
                                .is_ok()
                        {
                            log::info!("LEADER DELETED");
                        }
                    }
                }
                ApActivity::Like(like) => {
                    let id = like.id.ok_or(TaskError::TaskFailed)?;
                    log::debug!("LIKE ID: {id}");
                    let identifier = get_local_identifier(id).ok_or(TaskError::TaskFailed)?;
                    log::debug!("LIKE IDENTIFIER: {identifier:#?}");
                    if identifier.kind == LocalIdentifierType::Activity {
                        revoke_activity_by_uuid(Some(&conn), identifier.identifier)
                            .await
                            .map_err(|e| {
                                log::error!("LIKE REVOCATION FAILED: {e:#?}");
                                TaskError::TaskFailed
                            })?;
                    };
                }

                ApActivity::Announce(announce) => {
                    let id = announce.id.ok_or(TaskError::TaskFailed)?;
                    log::debug!("ANNOUNCE ID: {id}");
                    let identifier = get_local_identifier(id).ok_or(TaskError::TaskFailed)?;
                    log::debug!("ANNOUNCE IDENTIFIER: {identifier:#?}");
                    if identifier.kind == LocalIdentifierType::Activity {
                        revoke_activity_by_uuid(Some(&conn), identifier.identifier)
                            .await
                            .map_err(|e| {
                                log::error!("ANNOUNCE REVOCATION FAILED: {e:#?}");
                                TaskError::TaskFailed
                            })?;
                    }
                }
                _ => {
                    log::error!("FAILED TO MATCH REVOCABLE ACTIVITY");
                    return Err(TaskError::TaskFailed);
                }
            }
        }

        Ok(())
    }
}

impl TryFrom<ExtendedActivity> for ApUndo {
    type Error = anyhow::Error;

    fn try_from(
        (activity, target_activity, target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        let target_activity = target_activity.ok_or(anyhow!("RECURSIVE CANNOT BE NONE"))?;
        let target_activity =
            ApActivity::try_from((target_activity.clone(), None, target_object, None))?;

        if !activity.kind.is_undo() {
            return Err(anyhow!("activity is not an undo"));
        }

        match target_activity {
            ApActivity::Follow(follow) => Ok(ApUndo {
                context: Some(ApContext::default()),
                kind: ApUndoType::default(),
                actor: activity.actor.clone().into(),
                id: activity.ap_id,
                object: MaybeReference::Actual(ApActivity::Follow(follow)),
            }),
            ApActivity::Like(like) => Ok(ApUndo {
                context: Some(ApContext::default()),
                kind: ApUndoType::default(),
                actor: activity.actor.clone().into(),
                id: activity.ap_id,
                object: MaybeReference::Actual(ApActivity::Like(like)),
            }),
            ApActivity::Announce(announce) => Ok(ApUndo {
                context: Some(ApContext::default()),
                kind: ApUndoType::default(),
                actor: activity.actor.clone().into(),
                id: activity.ap_id,
                object: MaybeReference::Actual(ApActivity::Announce(announce)),
            }),
            _ => {
                log::error!("FAILED TO MATCH IMPLEMENTED UNDO: {activity:#?}");
                Err(anyhow!("FAILED TO MATCH IMPLEMENTED UNDO"))
            }
        }
    }
}

impl From<ApFollow> for ApUndo {
    fn from(follow: ApFollow) -> Self {
        ApUndo {
            context: Some(ApContext::default()),
            kind: ApUndoType::default(),
            actor: follow.actor.clone(),
            id: follow.id.clone().map(|follow| format!("{}#undo", follow)),
            object: MaybeReference::Actual(ApActivity::Follow(follow)),
        }
    }
}
