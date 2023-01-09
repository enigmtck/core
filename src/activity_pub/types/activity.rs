use crate::activity_pub::{
    ApActivityType, ApContext, ApEncryptedMessage, ApFlexible, ApInstrument, ApNote, ApObject,
    ApSession,
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
    pub uuid: Option<String>,
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
            uuid: Option::None,
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
        let mut note = note;
        note.context = Option::None;
        let uuid = Uuid::new_v4().to_string();

        if let Some(ApFlexible::Single(attributed_to)) = note.clone().attributed_to {
            let attributed_to = attributed_to.as_str().unwrap();

            note.id = Option::from(format!("{}/posts/{}", attributed_to, uuid));

            ApActivity {
                kind: ApActivityType::Create,
                id: note.clone().id,
                uuid: Option::from(uuid),
                to: Option::from(note.clone().to),
                actor: attributed_to.to_string(),
                object: ApObject::Note(note),
                ..Default::default()
            }
        } else {
            ApActivity {
                ..Default::default()
            }
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

impl From<ApEncryptedMessage> for ApActivity {
    fn from(encrypted_message: ApEncryptedMessage) -> Self {
        let mut encrypted_message = encrypted_message;
        encrypted_message.context = Option::None;
        let uuid = Uuid::new_v4().to_string();

        let attributed_to = encrypted_message.attributed_to.clone();

        encrypted_message.id = format!(
            "https://{}/encrypted-messages/{}",
            *crate::SERVER_NAME,
            uuid
        );

        ApActivity {
            kind: ApActivityType::Create,
            id: Option::from(format!("{}#encrypted-message", encrypted_message.id)),
            uuid: Option::from(uuid),
            to: Option::from(encrypted_message.to.clone()),
            actor: attributed_to,
            object: ApObject::EncryptedMessage(encrypted_message),
            ..Default::default()
        }
    }
}
