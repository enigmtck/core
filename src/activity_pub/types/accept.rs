use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApFollow, ApObject},
    MaybeReference,
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApAcceptType {
    #[default]
    #[serde(alias = "accept")]
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

impl TryFrom<ApFollow> for ApAccept {
    type Error = anyhow::Error;

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
            Err(anyhow!("COULD NOT IDENTIFY ACTOR"))
        }
    }
}
