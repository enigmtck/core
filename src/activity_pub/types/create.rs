use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApAddress, ApContext, ApNote, ApObject},
    models::activities::ExtendedActivity,
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
    pub actor: ApAddress,
    pub to: MaybeMultiple<ApAddress>,
    pub cc: Option<MaybeMultiple<ApAddress>>,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
    pub published: Option<String>,
    pub signature: Option<ApSignature>,
}

impl TryFrom<ExtendedActivity> for ApCreate {
    type Error = &'static str;

    fn try_from(
        (activity, note, _remote_note, _profile, _remote_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if let Some(note) = note {
            if let Some(ap_to) = activity.ap_to {
                Ok(ApCreate {
                    context: Some(ApContext::default()),
                    kind: ApCreateType::default(),
                    actor: ApAddress::Address(activity.actor.clone()),
                    id: Some(format!(
                        "{}/activities/{}",
                        *crate::SERVER_URL,
                        activity.uuid
                    )),
                    object: ApObject::Note(ApNote::from(note)).into(),
                    to: serde_json::from_value(ap_to).unwrap(),
                    cc: activity.cc.map(|cc| serde_json::from_value(cc).unwrap()),
                    signature: None,
                    published: Some(activity.created_at.to_rfc3339()),
                })
            } else {
                log::error!("ACTIVITY DOES NOT HAVE A TO FIELD");
                Err("ACTIVITY DOES NOT HAVE A TO FIELD")
            }
        } else {
            log::error!("ACTIVITY MUST INCLUDE A LOCALLY CREATED NOTE");
            Err("ACTIVITY MUST INCLUDE A LOCALLY CREATED NOTE")
        }
    }
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
