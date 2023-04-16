use core::fmt;
use std::fmt::Debug;

use crate::{
    //activity_pub::{ApActivity, ApActivityType, ApContext, ApFollow, ApObject},
    activity_pub::{ApActivity, ApContext, ApFollow, ApObject},
    MaybeReference,
};
use serde::{Deserialize, Serialize};

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
    pub actor: String,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
}

// impl TryFrom<ApActivity> for ApUndo {
//     type Error = &'static str;

//     fn try_from(activity: ApActivity) -> Result<Self, Self::Error> {
//         if activity.kind == ApActivityType::Undo {
//             Ok(ApUndo {
//                 context: activity.context,
//                 kind: ApUndoType::default(),
//                 actor: activity.actor,
//                 id: activity.id,
//                 object: activity.object,
//             })
//         } else {
//             Err("ACTIVITY COULD NOT BE CONVERTED TO UNDO")
//         }
//     }
// }

impl From<ApFollow> for ApUndo {
    fn from(follow: ApFollow) -> Self {
        ApUndo {
            context: Some(ApContext::default()),
            kind: ApUndoType::default(),
            actor: follow.actor.clone(),
            id: follow.id.clone().map(|follow| format!("{}#undo", follow)),
            object: MaybeReference::Actual(ApObject::Follow(Box::new(follow))),
        }
    }
}
