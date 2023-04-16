use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApAddress, ApContext, ApNote, ApObject},
    MaybeMultiple, MaybeReference,
};
use serde::{Deserialize, Serialize};

use super::signature::ApSignature;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApCreateType {
    #[default]
    Create,
}

impl fmt::Display for ApCreateType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApCreate {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApCreateType,
    pub actor: String,
    pub to: MaybeMultiple<ApAddress>,
    pub cc: Option<MaybeMultiple<ApAddress>>,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
    pub published: Option<String>,
    pub signature: Option<ApSignature>,
}

impl From<ApNote> for ApCreate {
    fn from(note: ApNote) -> Self {
        ApCreate {
            context: Some(ApContext::default()),
            kind: ApCreateType::default(),
            actor: note.attributed_to.clone(),
            id: note.id.clone().map(|id| format!("{id}#create")),
            object: ApObject::Note(note.clone()).into(),
            to: note.to.clone(),
            cc: note.cc.clone(),
            signature: None,
            published: note.published,
        }
    }
}
