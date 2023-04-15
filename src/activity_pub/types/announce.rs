use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApActivityType, ApAddress, ApContext, ApObject},
    models::announces::Announce,
    MaybeMultiple, MaybeReference,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApAnnounceType {
    #[default]
    Announce,
}

impl fmt::Display for ApAnnounceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

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
    pub cc: Option<MaybeMultiple<ApAddress>>,
    pub object: String,
}

impl From<Announce> for ApAnnounce {
    fn from(announce: Announce) -> Self {
        ApAnnounce {
            context: Some(ApContext::default()),
            kind: ApAnnounceType::default(),
            actor: announce.actor.into(),
            id: Some(format!(
                "{}/announces/{}",
                *crate::SERVER_URL,
                announce.uuid
            )),
            object: announce.object_ap_id,
            to: serde_json::from_value(announce.ap_to).unwrap(),
            cc: announce.cc.map(|cc| serde_json::from_value(cc).unwrap()),
        }
    }
}

impl TryFrom<ApActivity> for ApAnnounce {
    type Error = &'static str;

    fn try_from(activity: ApActivity) -> Result<Self, Self::Error> {
        if let MaybeReference::Reference(object_id) = activity.object {
            if activity.kind == ApActivityType::Announce {
                Ok(ApAnnounce {
                    context: Some(ApContext::default()),
                    kind: ApAnnounceType::default(),
                    actor: ApAddress::Address(activity.actor),
                    id: activity.id,
                    object: object_id,
                    to: activity.to.unwrap(),
                    cc: activity.cc.map(|cc| {
                        MaybeMultiple::Multiple(cc.iter().map(|cc| cc.clone().into()).collect())
                    }),
                })
            } else {
                Err("ACTIVITY IS NOT AN ANNOUNCE")
            }
        } else {
            Err("ACTIVITY OBJECT IS NOT PLAIN")
        }
    }
}
