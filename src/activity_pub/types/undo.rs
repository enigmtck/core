use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApFollow, Inbox, Outbox},
    db::Db,
    fairings::events::EventChannels,
    helper::{get_activity_ap_id_from_uuid, get_ap_id_from_username, get_uuid},
    models::{
        activities::{
            create_activity, get_activity_by_apid, get_activity_by_uuid, ActivityTarget,
            ActivityType, ApActivityTarget, NewActivity,
        },
        profiles::Profile,
    },
    runner, MaybeReference,
};
use anyhow::anyhow;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::activity::RecursiveActivity;

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

fn undo_target_apid(activity: &ApActivity) -> Option<String> {
    match activity {
        ApActivity::Like(like) => like.id.clone(),
        ApActivity::Follow(follow) => follow.id.clone(),
        ApActivity::Announce(announce) => announce.id.clone(),
        _ => None,
    }
}

async fn process_undo_activity(
    conn: Db,
    channels: EventChannels,
    ap_target: &ApActivity,
    undo: &ApUndo,
) -> Result<Status, Status> {
    let apid = undo_target_apid(ap_target).ok_or(Status::NotImplemented)?;
    log::debug!("APID: {apid}");
    // retrieve the activity to undo from the database (models/activities)
    let target = get_activity_by_apid(&conn, apid.clone())
        .await
        .ok_or(Status::NotFound)?;
    log::debug!("TARGET: {target:#?}");
    // set up the parameters necessary to create an Activity in the database with linked
    // target activity; NewActivity::try_from creates the link given the appropriate database
    // in the parameterized enum
    let activity_and_target = (
        ApActivity::Undo(Box::new(undo.clone())),
        Some(ActivityTarget::from(target.0)),
    ) as ApActivityTarget;

    let activity = NewActivity::try_from(activity_and_target).map_err(|_| Status::new(522))?;
    log::debug!("ACTIVITY\n{activity:#?}");
    if create_activity(Some(&conn), activity.clone()).await.is_ok() {
        match ap_target {
            ApActivity::Like(_) => {
                runner::run(
                    runner::like::process_remote_undo_like_task,
                    Some(conn),
                    Some(channels),
                    vec![apid.clone()],
                )
                .await;
                Ok(Status::Accepted)
            }
            ApActivity::Follow(_) => {
                runner::run(
                    runner::follow::process_remote_undo_follow_task,
                    Some(conn),
                    Some(channels),
                    vec![apid.clone()],
                )
                .await;
                Ok(Status::Accepted)
            }
            ApActivity::Announce(_) => {
                runner::run(
                    runner::announce::remote_undo_announce_task,
                    Some(conn),
                    Some(channels),
                    vec![apid.clone()],
                )
                .await;
                Ok(Status::Accepted)
            }
            _ => Err(Status::new(523)),
        }
    } else {
        Err(Status::new(524))
    }
}

impl Inbox for Box<ApUndo> {
    async fn inbox(&self, conn: Db, channels: EventChannels, raw: Value) -> Result<Status, Status> {
        match self.object.clone() {
            MaybeReference::Actual(actual) => {
                process_undo_activity(conn, channels, &actual, self).await
            }
            MaybeReference::Reference(_) => {
                log::warn!(
                    "INSUFFICIENT CONTEXT FOR UNDO TARGET (REFERENCE FOUND WHEN ACTUAL IS REQUIRED)"
                );
                log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                Err(Status::NoContent)
            }
            _ => {
                log::warn!(
                    "INSUFFICIENT CONTEXT FOR UNDO TARGET (NONE FOUND WHEN ACTUAL IS REQUIRED)"
                );
                log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                Err(Status::NoContent)
            }
        }
    }
}

impl Outbox for Box<ApUndo> {
    async fn outbox(
        &self,
        conn: Db,
        events: EventChannels,
        profile: Profile,
    ) -> Result<String, Status> {
        handle_undo(conn, events, *self.clone(), profile).await
    }
}

async fn handle_undo(
    conn: Db,
    channels: EventChannels,
    undo: ApUndo,
    profile: Profile,
) -> Result<String, Status> {
    let target_ap_id = match undo.object {
        MaybeReference::Actual(object) => match object {
            ApActivity::Follow(follow) => follow.id.and_then(get_uuid),
            ApActivity::Like(like) => like.id.and_then(get_uuid),
            ApActivity::Announce(announce) => announce.id.and_then(get_uuid),
            _ => None,
        },
        _ => None,
    };

    log::debug!("TARGET_AP_ID: {target_ap_id:#?}");
    if let Some(target_ap_id) = target_ap_id {
        if let Some((target_activity, _, _, _, _, _)) =
            get_activity_by_uuid(Some(&conn), target_ap_id).await
        {
            if let Ok(activity) = create_activity(
                Some(&conn),
                NewActivity::from((
                    target_activity,
                    ActivityType::Undo,
                    ApAddress::Address(get_ap_id_from_username(profile.username.clone())),
                ))
                .link_profile(&conn)
                .await,
            )
            .await
            {
                runner::run(
                    runner::undo::process_outbound_undo_task,
                    Some(conn),
                    Some(channels),
                    vec![activity.uuid.clone()],
                )
                .await;
                Ok(get_activity_ap_id_from_uuid(activity.uuid))
            } else {
                log::error!("FAILED TO CREATE UNDO ACTIVITY");
                Err(Status::NoContent)
            }
        } else {
            log::error!("FAILED TO RETRIEVE TARGET ACTIVITY");
            Err(Status::NoContent)
        }
    } else {
        log::error!("FAILED TO CONVERT OBJECT TO RELEVANT ACTIVITY");
        Err(Status::NoContent)
    }
}

impl TryFrom<RecursiveActivity> for ApUndo {
    type Error = anyhow::Error;

    fn try_from(
        ((activity, _note, _remote_note, _profile, _remote_actor, remote_question), recursive): RecursiveActivity,
    ) -> Result<Self, Self::Error> {
        if let Some(recursive) = recursive {
            if let Ok(recursive_activity) = ApActivity::try_from((recursive.clone(), None)) {
                match recursive_activity {
                    ApActivity::Follow(follow) => Ok(ApUndo {
                        context: Some(ApContext::default()),
                        kind: ApUndoType::default(),
                        actor: follow.actor.clone(),
                        id: Some(format!(
                            "{}/activities/{}",
                            *crate::SERVER_URL,
                            activity.uuid
                        )),
                        object: MaybeReference::Actual(ApActivity::Follow(follow)),
                    }),
                    ApActivity::Like(like) => Ok(ApUndo {
                        context: Some(ApContext::default()),
                        kind: ApUndoType::default(),
                        actor: like.actor.clone(),
                        id: Some(format!(
                            "{}/activities/{}",
                            *crate::SERVER_URL,
                            activity.uuid
                        )),
                        object: MaybeReference::Actual(ApActivity::Like(like)),
                    }),
                    ApActivity::Announce(announce) => Ok(ApUndo {
                        context: Some(ApContext::default()),
                        kind: ApUndoType::default(),
                        actor: announce.actor.clone(),
                        id: Some(format!(
                            "{}/activities/{}",
                            *crate::SERVER_URL,
                            activity.uuid
                        )),
                        object: MaybeReference::Actual(ApActivity::Announce(announce)),
                    }),
                    _ => {
                        log::error!("FAILED TO MATCH IMPLEMENTED UNDO: {activity:#?}");
                        Err(anyhow!("FAILED TO MATCH IMPLEMENTED UNDO"))
                    }
                }
            } else {
                log::error!("FAILED TO CONVERT ACTIVITY: {recursive:#?}");
                Err(anyhow!("FAILED TO CONVERT ACTIVITY"))
            }
        } else {
            log::error!("RECURSIVE CANNOT BE NONE");
            Err(anyhow!("RECURSIVE CANNOT BE NONE"))
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
