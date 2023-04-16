use core::fmt;
use std::fmt::Debug;

use crate::{
    // activity_pub::{ApActivity, ApActivityType, ApContext},
    activity_pub::{ApActivity, ApContext},
    models::likes::Like,
    MaybeReference,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApLikeType {
    #[default]
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
    pub actor: String,
    pub id: Option<String>,
    pub object: String,
}

impl From<Like> for ApLike {
    fn from(like: Like) -> Self {
        ApLike {
            context: Some(ApContext::default()),
            kind: ApLikeType::Like,
            actor: like.actor,
            id: Some(format!("{}/likes/{}", *crate::SERVER_URL, like.uuid)),
            object: like.object_ap_id,
        }
    }
}

// impl TryFrom<ApActivity> for ApLike {
//     type Error = &'static str;

//     fn try_from(activity: ApActivity) -> Result<Self, Self::Error> {
//         if let MaybeReference::Reference(object_id) = activity.object {
//             if activity.kind == ApActivityType::Like {
//                 Ok(ApLike {
//                     context: Some(ApContext::default()),
//                     kind: ApLikeType::default(),
//                     actor: activity.actor,
//                     id: activity.id,
//                     object: object_id,
//                 })
//             } else {
//                 Err("ACTIVITY IS NOT A LIKE")
//             }
//         } else {
//             Err("ACTIVITY OBJECT IS NOT PLAIN")
//         }
//     }
// }
