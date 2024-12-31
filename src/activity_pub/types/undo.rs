use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApFollow},
    models::activities::ExtendedActivity,
    MaybeReference,
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

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
