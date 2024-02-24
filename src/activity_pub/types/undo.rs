use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApFollow, Inbox, Outbox},
    db::Db,
    fairings::{events::EventChannels, faktory::FaktoryConnection},
    models::{
        activities::{
            create_activity, get_activity_by_apid, ActivityTarget, ApActivityTarget, NewActivity,
        },
        profiles::Profile,
    },
    outbox, to_faktory, MaybeReference,
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::activity::RecursiveActivity;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApUndoType {
    #[default]
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
    conn: &Db,
    faktory: FaktoryConnection,
    ap_target: &ApActivity,
    undo: &ApUndo,
) -> Result<Status, Status> {
    if let Some(ref apid) = undo_target_apid(ap_target) {
        log::debug!("APID: {apid}");
        // retrieve the activity to undo from the database (models/activities)
        if let Some(target) = get_activity_by_apid(conn, apid.clone()).await {
            log::debug!("TARGET: {target:#?}");
            // set up the parameters necessary to create an Activity in the database with linked
            // target activity; NewActivity::try_from creates the link given the appropriate database
            // in the parameterized enum
            let activity_and_target = (
                ApActivity::Undo(Box::new(undo.clone())),
                Some(ActivityTarget::from(target.0)),
            ) as ApActivityTarget;

            if let Ok(activity) = NewActivity::try_from(activity_and_target) {
                log::debug!("ACTIVITY\n{activity:#?}");
                if create_activity(conn.into(), activity.clone())
                    .await
                    .is_some()
                {
                    match ap_target {
                        ApActivity::Like(_) => {
                            to_faktory(faktory, "process_remote_undo_like", apid.clone())
                        }
                        ApActivity::Follow(_) => {
                            to_faktory(faktory, "process_remote_undo_follow", apid.clone())
                        }
                        ApActivity::Announce(_) => {
                            to_faktory(faktory, "process_remote_undo_announce", apid.clone())
                        }
                        _ => Err(Status::NoContent),
                    }
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

impl Inbox for Box<ApUndo> {
    async fn inbox(
        &self,
        conn: Db,
        faktory: FaktoryConnection,
        raw: Value,
    ) -> Result<Status, Status> {
        match self.object.clone() {
            MaybeReference::Actual(actual) => {
                process_undo_activity(&conn, faktory, &actual, self).await
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
        faktory: FaktoryConnection,
        _events: EventChannels,
        profile: Profile,
    ) -> Result<String, Status> {
        outbox::activity::undo(conn, faktory, *self.clone(), profile).await
    }
}

impl TryFrom<RecursiveActivity> for ApUndo {
    type Error = &'static str;

    fn try_from(
        ((activity, _note, _remote_note, _profile, _remote_actor), recursive): RecursiveActivity,
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
                        Err("FAILED TO MATCH IMPLEMENTED UNDO")
                    }
                }
            } else {
                log::error!("FAILED TO CONVERT ACTIVITY: {recursive:#?}");
                Err("FAILED TO CONVERT ACTIVITY")
            }
        } else {
            log::error!("RECURSIVE CANNOT BE NONE");
            Err("RECURSIVE CANNOT BE NONE")
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
