use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApFollow},
    MaybeReference,
};
use serde::{Deserialize, Serialize};

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
