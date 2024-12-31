use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApAddress, ApContext, ApNote, ApObject},
    models::coalesced_activity::CoalescedActivity,
    MaybeMultiple, MaybeReference,
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use super::signature::ApSignature;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApDeleteType {
    #[default]
    #[serde(alias = "delete")]
    Delete,
}

impl fmt::Display for ApDeleteType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApDelete {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApDeleteType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
    pub signature: Option<ApSignature>,
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub to: MaybeMultiple<ApAddress>,
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub cc: MaybeMultiple<ApAddress>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApTombstoneType {
    #[default]
    Tombstone,
}

impl fmt::Display for ApTombstoneType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApTombstone {
    #[serde(rename = "type")]
    pub kind: ApTombstoneType,
    pub id: String,
    pub atom_uri: Option<String>,
}

impl TryFrom<ApNote> for ApTombstone {
    type Error = anyhow::Error;

    fn try_from(note: ApNote) -> Result<Self, Self::Error> {
        if let Some(id) = note.id {
            Ok(ApTombstone {
                kind: ApTombstoneType::Tombstone,
                id: id.clone(),
                atom_uri: Some(id),
            })
        } else {
            Err(anyhow!("ApNote must have an ID"))
        }
    }
}

impl TryFrom<ApNote> for ApDelete {
    type Error = anyhow::Error;

    fn try_from(note: ApNote) -> Result<Self, Self::Error> {
        note.id.clone().ok_or(anyhow!("ApNote must have an ID"))?;
        let tombstone = ApTombstone::try_from(note.clone())?;
        Ok(ApDelete {
            context: Some(ApContext::default()),
            actor: note.attributed_to.clone(),
            kind: ApDeleteType::Delete,
            id: None, // This will be set in NewActivity
            object: MaybeReference::Actual(ApObject::Tombstone(tombstone)),
            signature: None,
            to: note.to,
            cc: note.cc,
        })
    }
}

impl TryFrom<CoalescedActivity> for ApDelete {
    type Error = anyhow::Error;

    fn try_from(activity: CoalescedActivity) -> Result<Self, Self::Error> {
        if !activity.kind.is_delete() {
            return Err(anyhow!("Not a Delete Activity"));
        }

        Ok(ApDelete {
            context: Some(ApContext::default()),
            kind: ApDeleteType::default(),
            actor: activity.actor.into(),
            id: activity.ap_id,
            to: activity.ap_to.into(),
            cc: activity.cc.into(),
            object: activity
                .object_as_id
                .ok_or(anyhow!("no object_as_id"))?
                .into(),
            signature: None,
        })
    }
}
