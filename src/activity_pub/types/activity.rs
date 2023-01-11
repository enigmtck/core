use crate::{
    activity_pub::{ApActivityType, ApContext, ApInstrument, ApNote, ApObject, ApSession},
    models::notes::NewNote,
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt::Debug;
use uuid::Uuid;

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApActivity {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApActivityType,
    pub id: Option<String>,
    pub actor: String,
    pub to: Option<Vec<String>>,
    pub cc: Option<Vec<String>>,
    pub published: Option<String>,
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
            object: ApObject::default(),
        }
    }
}

impl From<ApNote> for ApActivity {
    fn from(note: ApNote) -> Self {
        ApActivity {
            kind: ApActivityType::Create,
            id: Some(format!("{}#create", note.clone().id.unwrap())),
            to: Option::from(note.clone().to),
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

        if let ApInstrument::Multiple(_) = session.instrument {
            kind = ApActivityType::Join;
        }

        ApActivity {
            id: Option::from(format!("{}#join", session.id.clone().unwrap_or_default())),
            kind,
            to: Option::from(vec![session.to.clone()]),
            actor: session.attributed_to.clone(),
            object: ApObject::Session(session),
            ..Default::default()
        }
    }
}
