use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApAddress, ApContext, ApObject},
    models::{activities::ExtendedActivity, coalesced_activity::CoalescedActivity},
    MaybeMultiple, MaybeReference,
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApFollowType {
    #[default]
    #[serde(alias = "follow")]
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
    pub actor: ApAddress,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub to: MaybeMultiple<ApAddress>,
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub cc: MaybeMultiple<ApAddress>,
    pub object: MaybeReference<ApObject>,
}

impl TryFrom<ExtendedActivity> for ApFollow {
    type Error = anyhow::Error;

    fn try_from(
        (activity, _target_activity, _target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if activity.kind.is_follow() {
            let target = activity
                .target_ap_id
                .ok_or(anyhow!("no target_ap_id on follow"))?;
            Ok(ApFollow {
                context: Some(ApContext::default()),
                kind: ApFollowType::default(),
                actor: activity.actor.into(),
                id: Some(activity.ap_id.ok_or(anyhow!("no follow as_id found"))?),
                to: activity.ap_to.into(),
                cc: activity.cc.into(),
                object: target.into(),
            })
        } else {
            log::error!("Not a Follow Activity");
            Err(anyhow!("Not a Follow Activity"))
        }
    }
}

impl TryFrom<CoalescedActivity> for ApFollow {
    type Error = anyhow::Error;

    fn try_from(activity: CoalescedActivity) -> Result<Self, Self::Error> {
        if activity.kind.is_follow() {
            Ok(ApFollow {
                context: Some(ApContext::default()),
                kind: ApFollowType::default(),
                actor: activity.actor.into(),
                id: Some(activity.ap_id.ok_or(anyhow!("no follow as_id found"))?),
                to: activity.ap_to.into(),
                cc: activity.cc.into(),
                object: activity
                    .object_as_id
                    .ok_or(anyhow!("no object_as_id"))?
                    .into(),
            })
        } else {
            log::error!("Not a Follow Activity");
            Err(anyhow!("Not a Follow Activity"))
        }
    }
}
