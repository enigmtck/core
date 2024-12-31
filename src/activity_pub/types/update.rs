use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActor, ApAddress, ApContext, ApNote, ApObject},
    MaybeMultiple, MaybeReference,
};
use serde::{Deserialize, Serialize};

use super::signature::ApSignature;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApUpdateType {
    #[default]
    #[serde(alias = "update")]
    Update,
}

impl fmt::Display for ApUpdateType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApUpdate {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApUpdateType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
    pub signature: Option<ApSignature>,
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub to: MaybeMultiple<ApAddress>,
}

impl TryFrom<ApNote> for ApUpdate {
    type Error = &'static str;

    fn try_from(note: ApNote) -> Result<Self, Self::Error> {
        if let Some(id) = note.id.clone() {
            Ok(ApUpdate {
                context: Some(ApContext::default()),
                actor: note.attributed_to.clone(),
                kind: ApUpdateType::default(),
                id: Some(format!("{id}#update")),
                object: MaybeReference::Actual(ApObject::Note(note.clone())),
                signature: None,
                to: note.to,
            })
        } else {
            Err("ApNote must have an ID")
        }
    }
}

impl TryFrom<ApActor> for ApUpdate {
    type Error = &'static str;

    fn try_from(actor: ApActor) -> Result<Self, Self::Error> {
        if let Some(id) = actor.id.clone() {
            Ok(ApUpdate {
                context: Some(ApContext::default()),
                actor: id.clone(),
                kind: ApUpdateType::default(),
                id: Some(format!("{id}#update")),
                object: MaybeReference::Actual(ApObject::Actor(actor)),
                signature: None,
                to: vec![ApAddress::get_public()].into(),
            })
        } else {
            Err("ApActor must have an ID")
        }
    }
}
