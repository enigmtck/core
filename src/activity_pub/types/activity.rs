use crate::{
    activity_pub::{ApContext, ApInstrument, ApNote, ApObject, ApSession},
    models::{remote_activities::RemoteActivity, remote_announces::RemoteAnnounce},
    MaybeMultiple,
};
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt::Debug;

use super::{
    actor::ApAddress,
    object::{ApProof, ApSignature},
    session::ApInstruments,
};

#[derive(Serialize, PartialEq, Eq, Deserialize, Clone, Debug, Default)]
pub enum ApActivityType {
    Create,
    Update,
    Delete,
    Follow,
    Accept,
    Reject,
    Add,
    Remove,
    Like,
    Announce,
    Undo,
    Invite,
    Join,
    #[default]
    Unknown,
}

impl fmt::Display for ApActivityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl From<String> for ApActivityType {
    fn from(data: String) -> Self {
        data.as_str().into()
    }
}

impl From<&str> for ApActivityType {
    fn from(data: &str) -> Self {
        match data {
            "Create" => ApActivityType::Create,
            "Update" => ApActivityType::Update,
            "Delete" => ApActivityType::Delete,
            "Follow" => ApActivityType::Follow,
            "Accept" => ApActivityType::Accept,
            "Reject" => ApActivityType::Reject,
            "Add" => ApActivityType::Add,
            "Remove" => ApActivityType::Remove,
            "Like" => ApActivityType::Like,
            "Announce" => ApActivityType::Announce,
            "Undo" => ApActivityType::Undo,
            "Invite" => ApActivityType::Invite,
            "Join" => ApActivityType::Join,
            _ => ApActivityType::Unknown,
        }
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApActivity {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApActivityType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub actor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<MaybeMultiple<ApAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<ApProof>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instrument: Option<ApInstrument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<ApSignature>,
    pub object: ApObject,
}

impl Default for ApActivity {
    fn default() -> ApActivity {
        ApActivity {
            context: Option::from(ApContext::Plain(
                "https://www.w3.org/ns/activitystreams".to_string(),
            )),
            kind: ApActivityType::default(),
            id: Option::None,
            actor: String::new(),
            to: Option::None,
            cc: Option::None,
            published: Option::None,
            proof: Option::None,
            instrument: Option::None,
            signature: Option::None,
            object: ApObject::default(),
        }
    }
}

impl From<ApNote> for ApActivity {
    fn from(note: ApNote) -> Self {
        ApActivity {
            kind: ApActivityType::Create,
            id: Some(format!("{}#create", note.clone().id.unwrap())),
            to: Some(note.clone().to),
            actor: note.clone().attributed_to,
            object: ApObject::Note(note),
            ..Default::default()
        }
    }
}

impl From<ApSession> for ApActivity {
    fn from(session: ApSession) -> Self {
        let mut session = session;
        session.context = Option::None;

        let mut kind = ApActivityType::Invite;

        if let ApInstruments::Multiple(_) = session.instrument {
            kind = ApActivityType::Join;
        }

        ApActivity {
            id: Option::from(format!("{}#join", session.id.clone().unwrap_or_default())),
            kind,
            to: Option::from(MaybeMultiple::Multiple(vec![ApAddress::Address(
                session.to.clone(),
            )])),
            actor: session.attributed_to.clone(),
            object: ApObject::Session(session),
            ..Default::default()
        }
    }
}

impl From<RemoteActivity> for ApActivity {
    fn from(activity: RemoteActivity) -> Self {
        ApActivity {
            context: Some(ApContext::Complex(vec![activity
                .context
                .unwrap_or_default()])),
            kind: activity.kind.into(),
            id: Option::from(activity.ap_id),
            actor: activity.actor,
            to: Option::from(
                serde_json::from_value::<MaybeMultiple<ApAddress>>(
                    activity.ap_to.unwrap_or_default(),
                )
                .unwrap(),
            ),
            cc: Option::from(
                serde_json::from_value::<Vec<String>>(activity.cc.unwrap_or_default()).unwrap(),
            ),
            published: activity.published,
            proof: Option::None,
            instrument: Option::None,
            signature: Option::None,
            object: serde_json::from_value(activity.ap_object.unwrap()).unwrap(),
        }
    }
}

impl From<RemoteAnnounce> for ApActivity {
    fn from(activity: RemoteAnnounce) -> Self {
        ApActivity {
            context: serde_json::from_str(&activity.context.unwrap()).unwrap(),
            kind: activity.kind.into(),
            id: Option::from(activity.ap_id),
            actor: activity.actor,
            to: Option::from(
                serde_json::from_value::<MaybeMultiple<ApAddress>>(
                    activity.ap_to.unwrap_or_default(),
                )
                .unwrap(),
            ),
            cc: Option::from(
                serde_json::from_value::<Vec<String>>(activity.cc.unwrap_or_default()).unwrap(),
            ),
            published: Some(activity.published),
            proof: Option::None,
            instrument: Option::None,
            signature: Option::None,
            object: serde_json::from_value(activity.ap_object).unwrap(),
        }
    }
}
