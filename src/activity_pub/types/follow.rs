use core::fmt;
use std::fmt::Debug;

use crate::{
    // activity_pub::{ApActivity, ApActivityType, ApContext, ApObject},
    activity_pub::{ApActivity, ApContext, ApObject},
    models::{follows::Follow, remote_activities::RemoteActivity},
    MaybeReference,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApFollowType {
    #[default]
    Follow,
}

impl fmt::Display for ApFollowType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApFollow {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApFollowType,
    pub actor: String,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
}

// impl TryFrom<ApActivity> for ApFollow {
//     type Error = &'static str;

//     fn try_from(activity: ApActivity) -> Result<Self, Self::Error> {
//         if activity.kind == ApActivityType::Follow {
//             Ok(ApFollow {
//                 context: activity.context,
//                 kind: ApFollowType::default(),
//                 actor: activity.actor,
//                 id: activity.id,
//                 object: activity.object,
//             })
//         } else {
//             Err("ACTIVITY COULD NOT BE CONVERTED TO FOLLOW")
//         }
//     }
// }

impl TryFrom<RemoteActivity> for ApFollow {
    type Error = &'static str;

    fn try_from(activity: RemoteActivity) -> Result<Self, Self::Error> {
        if activity.kind == "Follow" {
            Ok(ApFollow {
                context: activity
                    .context
                    .map(|ctx| serde_json::from_value(ctx).unwrap()),
                kind: ApFollowType::default(),
                actor: activity.actor,
                id: Some(activity.ap_id),
                object: serde_json::from_value(activity.ap_object.into()).unwrap(),
            })
        } else {
            Err("ACTIVITY COULD NOT BE CONVERTED TO ACCEPT")
        }
    }
}

impl From<Follow> for ApFollow {
    fn from(follow: Follow) -> Self {
        ApFollow {
            context: Some(ApContext::default()),
            kind: ApFollowType::default(),
            actor: follow.actor,
            id: Some(format!("{}/follows/{}", *crate::SERVER_URL, follow.uuid)),
            object: MaybeReference::Reference(follow.ap_object),
        }
    }
}
