use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ActivityPub, ApAddress, ApContext, ApNote, ApObject, Temporal},
    models::{
        activities::{ActivityType, ExtendedActivity},
        coalesced_activity::CoalescedActivity,
    },
    MaybeMultiple, MaybeReference,
};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::question::ApQuestion;
use super::Ephemeral;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApAnnounceType {
    #[default]
    #[serde(alias = "announce")]
    Announce,
}

impl fmt::Display for ApAnnounceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl TryFrom<ActivityType> for ApAnnounceType {
    type Error = anyhow::Error;

    fn try_from(t: ActivityType) -> Result<Self, Self::Error> {
        match t {
            ActivityType::Announce => Ok(ApAnnounceType::Announce),
            _ => Err(anyhow!("invalid ActivityType")),
        }
    }
}

// The sqlite version changes the ephemeral dates to naive, but I don't want to do that
// may need to fix this (Ap versions should be UTC while Db versions should be naive)
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApAnnounce {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApAnnounceType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub to: MaybeMultiple<ApAddress>,
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub cc: MaybeMultiple<ApAddress>,
    pub published: String,
    pub object: MaybeReference<ApObject>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<Ephemeral>,
}

impl Temporal for ApAnnounce {
    fn published(&self) -> String {
        self.published.clone()
    }

    fn created_at(&self) -> Option<DateTime<Utc>> {
        self.ephemeral.clone().and_then(|x| x.created_at)
    }

    fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.ephemeral.clone().and_then(|x| x.updated_at)
    }
}

impl TryFrom<CoalescedActivity> for ApAnnounce {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        let object = match coalesced
            .clone()
            .object_type
            .ok_or(anyhow!("object_type is None"))?
            .to_string()
            .to_lowercase()
            .as_str()
        {
            "note" => Ok(ApObject::Note(ApNote::try_from(coalesced.clone())?).into()),
            "question" => Ok(ApObject::Question(ApQuestion::try_from(coalesced.clone())?).into()),
            _ => Err(anyhow!("invalid type")),
        }?;
        let kind = coalesced.kind.clone().try_into()?;
        let actor = ApAddress::Address(coalesced.actor.clone());
        let id = coalesced.ap_id.clone();
        let context = Some(ApContext::default());
        let to = coalesced.clone().ap_to.into();
        let cc = coalesced.clone().cc.into();
        let published = ActivityPub::time(coalesced.created_at);
        let ephemeral = Some(Ephemeral {
            created_at: Some(coalesced.created_at),
            updated_at: Some(coalesced.updated_at),
            ..Default::default()
        });

        Ok(ApAnnounce {
            context,
            kind,
            actor,
            id,
            object,
            to,
            cc,
            published,
            ephemeral,
        })
    }
}

impl TryFrom<ExtendedActivity> for ApAnnounce {
    type Error = anyhow::Error;

    fn try_from(
        (activity, _target_activity, target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if activity.kind.to_string().to_lowercase().as_str() == "announce" {
            let object = target_object.ok_or(anyhow!("INVALID ACTIVITY TYPE"))?;
            let object = MaybeReference::Actual(ApObject::Note(ApNote::try_from(object)?));

            Ok(ApAnnounce {
                context: Some(ApContext::default()),
                kind: ApAnnounceType::default(),
                actor: activity.clone().actor.into(),
                id: Some(format!(
                    "{}/activities/{}",
                    *crate::SERVER_URL,
                    activity.uuid
                )),
                to: activity.clone().ap_to.into(),
                cc: activity.cc.into(),
                published: ActivityPub::time(activity.created_at),
                object,
                ephemeral: Some(Ephemeral {
                    created_at: Some(activity.created_at),
                    updated_at: Some(activity.updated_at),
                    ..Default::default()
                }),
            })
        } else {
            log::error!("NOT AN ANNOUNCE ACTIVITY");
            Err(anyhow::Error::msg("NOT AN ANNOUNCE ACTIVITY"))
        }
    }
}
