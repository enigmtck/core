use core::fmt;
use std::fmt::Debug;

use crate::{
    // activity_pub::{ApActivity, ApActivityType, ApContext},
    activity_pub::{ApAddress, ApContext, ApNote, ApObject},
    models::activities::{ActivityType, ExtendedActivity},
    MaybeMultiple,
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
    pub actor: ApAddress,
    #[serde(skip_serializing)]
    pub to: Option<MaybeMultiple<ApAddress>>,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
}

impl TryFrom<ExtendedActivity> for ApLike {
    type Error = &'static str;

    fn try_from(
        (activity, note, remote_note, profile, _remote_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if activity.kind == ActivityType::Like {
            match (note, remote_note, profile) {
                (Some(note), None, None) => Ok(ApLike {
                    context: Some(ApContext::default()),
                    kind: ApLikeType::default(),
                    actor: activity.actor.into(),
                    id: Some(format!(
                        "{}/activities/{}",
                        *crate::SERVER_URL,
                        activity.uuid
                    )),
                    to: Some(MaybeMultiple::Single(ApAddress::Address(
                        note.attributed_to.clone(),
                    ))),
                    object: MaybeReference::Reference(ApNote::from(note).id.unwrap()),
                }),
                (None, Some(remote_note), None) => Ok(ApLike {
                    context: Some(ApContext::default()),
                    kind: ApLikeType::default(),
                    actor: activity.actor.into(),
                    id: Some(format!(
                        "{}/activities/{}",
                        *crate::SERVER_URL,
                        activity.uuid
                    )),
                    to: Some(MaybeMultiple::Single(ApAddress::Address(
                        remote_note.attributed_to,
                    ))),
                    object: MaybeReference::Reference(remote_note.ap_id),
                }),
                _ => {
                    log::error!("INVALID ACTIVITY TYPE");
                    Err("INVALID ACTIVITY TYPE")
                }
            }
        } else {
            log::error!("NOT A LIKE ACTIVITY");
            Err("NOT A LIKE ACTIVITY")
        }
    }
}

// impl From<Like> for ApLike {
//     fn from(like: Like) -> Self {
//         ApLike {
//             context: Some(ApContext::default()),
//             kind: ApLikeType::Like,
//             actor: ApAddress::Address(like.actor),
//             to: Some(MaybeMultiple::Single(ApAddress::Address(like.ap_to))),
//             id: Some(format!("{}/likes/{}", *crate::SERVER_URL, like.uuid)),
//             object: MaybeReference::Reference(like.object_ap_id),
//         }
//     }
// }
