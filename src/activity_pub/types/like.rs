use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApAddress, ApContext, ApNote, ApObject},
    models::{activities::ExtendedActivity, coalesced_activity::CoalescedActivity},
    MaybeMultiple, MaybeReference,
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApLikeType {
    #[default]
    #[serde(alias = "like")]
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
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub to: MaybeMultiple<ApAddress>,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
}

impl TryFrom<ExtendedActivity> for ApLike {
    type Error = anyhow::Error;

    fn try_from(
        (activity, _target_activity, target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if !activity.kind.is_like() {
            return Err(anyhow!("NOT A LIKE ACTIVITY"));
        }

        let object = target_object.ok_or(anyhow!("no target object"))?;
        let note = ApNote::try_from(object)?;

        let (id, object): (String, MaybeReference<ApObject>) = (
            note.attributed_to.clone().to_string(),
            MaybeReference::Reference(note.id.ok_or(anyhow!("no note id"))?),
        );

        Ok(ApLike {
            context: Some(ApContext::default()),
            kind: ApLikeType::default(),
            actor: activity.actor.into(),
            id: activity.ap_id,
            to: MaybeMultiple::Single(ApAddress::Address(id)),
            object,
        })
    }
}

impl TryFrom<CoalescedActivity> for ApLike {
    type Error = anyhow::Error;

    fn try_from(activity: CoalescedActivity) -> Result<Self, Self::Error> {
        if !activity.kind.is_like() {
            return Err(anyhow!("Not a Like Activity"));
        }

        Ok(ApLike {
            context: Some(ApContext::default()),
            kind: ApLikeType::default(),
            actor: activity.actor.into(),
            id: activity.ap_id,
            to: activity.ap_to.into(),
            object: activity
                .object_as_id
                .ok_or(anyhow!("no object_as_id"))?
                .into(),
        })
    }
}
