use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApActivityType, ApContext, ApFollow, ApObject},
    MaybeReference,
};
use serde::{Deserialize, Serialize};

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
    pub actor: String,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
}

impl TryFrom<ApActivity> for ApAccept {
    type Error = &'static str;

    fn try_from(activity: ApActivity) -> Result<Self, Self::Error> {
        if activity.kind == ApActivityType::Accept {
            Ok(ApAccept {
                context: activity.context,
                kind: ApAcceptType::default(),
                actor: activity.actor,
                id: activity.id,
                object: activity.object,
            })
        } else {
            Err("ACTIVITY COULD NOT BE CONVERTED TO ACCEPT")
        }
    }
}

impl TryFrom<ApFollow> for ApAccept {
    type Error = &'static str;

    fn try_from(follow: ApFollow) -> Result<Self, Self::Error> {
        let actor = {
            match follow.object.clone() {
                MaybeReference::Actual(ApObject::Actor(actual)) => actual.id,
                MaybeReference::Reference(reference) => Some(reference),
                _ => None,
            }
        };

        if let Some(actor) = actor {
            Ok(ApAccept {
                context: Some(ApContext::default()),
                kind: ApAcceptType::default(),
                actor,
                id: follow.id.clone().map(|id| format!("{id}#accept")),
                object: MaybeReference::Actual(ApObject::Follow(Box::new(follow))),
            })
        } else {
            Err("COULD NOT IDENTIFY ACTOR")
        }
    }
}
