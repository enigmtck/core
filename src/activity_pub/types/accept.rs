use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApFollow, ApObject},
    //    models::remote_activities::RemoteActivity,
    MaybeReference,
};
use serde::{Deserialize, Serialize};

use super::activity::RecursiveActivity;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApAcceptType {
    #[default]
    Accept,
}

impl fmt::Display for ApAcceptType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApAccept {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApAcceptType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub object: MaybeReference<ApActivity>,
}

impl TryFrom<RecursiveActivity> for ApAccept {
    type Error = &'static str;

    fn try_from(
        ((activity, _note, _remote_note, _profile, _remote_actor), recursive): RecursiveActivity,
    ) -> Result<Self, Self::Error> {
        if let Some(recursive) = recursive {
            if let Ok(recursive_activity) = ApActivity::try_from((recursive.clone(), None)) {
                match recursive_activity {
                    ApActivity::Follow(follow) => Ok(ApAccept {
                        context: Some(ApContext::default()),
                        kind: ApAcceptType::default(),
                        actor: activity.actor.clone().into(),
                        id: Some(format!(
                            "{}/activities/{}",
                            *crate::SERVER_URL,
                            activity.uuid
                        )),
                        object: MaybeReference::Actual(ApActivity::Follow(follow)),
                    }),
                    _ => {
                        log::error!("FAILED TO MATCH IMPLEMENTED ACCEPT: {activity:#?}");
                        Err("FAILED TO MATCH IMPLEMENTED ACCEPT")
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

// impl TryFrom<RemoteActivity> for ApAccept {
//     type Error = &'static str;

//     fn try_from(activity: RemoteActivity) -> Result<Self, Self::Error> {
//         if activity.kind == "Accept" {
//             Ok(ApAccept {
//                 context: activity
//                     .context
//                     .map(|ctx| serde_json::from_value(ctx).unwrap()),
//                 kind: ApAcceptType::default(),
//                 actor: ApAddress::Address(activity.actor),
//                 id: Some(activity.ap_id),
//                 object: serde_json::from_value(activity.ap_object.into()).unwrap(),
//             })
//         } else {
//             Err("ACTIVITY COULD NOT BE CONVERTED TO ACCEPT")
//         }
//     }
// }

impl TryFrom<ApFollow> for ApAccept {
    type Error = &'static str;

    fn try_from(follow: ApFollow) -> Result<Self, Self::Error> {
        let actor = {
            match follow.object.clone() {
                MaybeReference::Actual(ApObject::Actor(actual)) => actual.id,
                MaybeReference::Reference(reference) => Some(ApAddress::Address(reference)),
                _ => None,
            }
        };

        if let Some(actor) = actor {
            Ok(ApAccept {
                context: Some(ApContext::default()),
                kind: ApAcceptType::default(),
                actor,
                id: follow.id.clone().map(|id| format!("{id}#accept")),
                object: MaybeReference::Actual(ApActivity::Follow(follow)),
            })
        } else {
            Err("COULD NOT IDENTIFY ACTOR")
        }
    }
}
