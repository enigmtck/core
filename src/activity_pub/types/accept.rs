use core::fmt;
use std::fmt::Debug;

use crate::{
    //activity_pub::{ApActivity, ApActivityType, ApContext, ApFollow, ApObject},
    activity_pub::{ApActivity, ApContext, ApFollow, ApObject},
    models::remote_activities::RemoteActivity,
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

impl TryFrom<RemoteActivity> for ApAccept {
    type Error = &'static str;

    fn try_from(activity: RemoteActivity) -> Result<Self, Self::Error> {
        if activity.kind == "Accept" {
            Ok(ApAccept {
                context: activity
                    .context
                    .map(|ctx| serde_json::from_value(ctx).unwrap()),
                kind: ApAcceptType::default(),
                actor: activity.actor,
                id: Some(activity.ap_id),
                object: serde_json::from_value(activity.ap_object.into()).unwrap(),
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
