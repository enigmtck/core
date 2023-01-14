use crate::{
    activity_pub::{
        ApBasicContent, ApBasicContentType, ApContext, ApInstrument, ApObject, ApObjectType,
    },
    models::{
        encrypted_sessions::EncryptedSession, remote_encrypted_sessions::RemoteEncryptedSession,
    },
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApSession {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApObjectType,
    pub id: Option<String>,
    pub to: String,
    pub attributed_to: String,
    pub instrument: ApInstrument,
    pub reference: Option<String>,
}

impl Default for ApSession {
    fn default() -> ApSession {
        ApSession {
            context: Option::from(ApContext::Plain(
                "https://www.w3.org/ns/activitystreams".to_string(),
            )),
            kind: ApObjectType::EncryptedSession,
            id: Option::from(format!(
                "https://{}/encrypted-sessions/{}",
                *crate::SERVER_NAME,
                Uuid::new_v4()
            )),
            to: String::new(),
            attributed_to: String::new(),
            instrument: ApInstrument::default(),
            reference: Option::None,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct JoinData {
    pub one_time_key: String,
    pub identity_key: String,
    pub to: String,
    pub attributed_to: String,
    pub reference: String,
}

impl From<JoinData> for ApSession {
    fn from(keys: JoinData) -> ApSession {
        ApSession {
            reference: Option::from(keys.reference),
            to: keys.to,
            attributed_to: keys.attributed_to,
            instrument: ApInstrument::Multiple(vec![
                ApObject::Basic(ApBasicContent {
                    kind: ApBasicContentType::IdentityKey,
                    content: keys.identity_key,
                }),
                ApObject::Basic(ApBasicContent {
                    kind: ApBasicContentType::SessionKey,
                    content: keys.one_time_key,
                }),
            ]),
            ..Default::default()
        }
    }
}

impl From<EncryptedSession> for ApSession {
    fn from(session: EncryptedSession) -> ApSession {
        ApSession {
            id: Option::from(format!(
                "https://{}/encrypted-sessions/{}",
                *crate::SERVER_NAME,
                session.uuid
            )),
            reference: session.reference,
            to: session.ap_to,
            attributed_to: session.attributed_to,
            instrument: serde_json::from_value(session.instrument).unwrap(),

            ..Default::default()
        }
    }
}

impl From<RemoteEncryptedSession> for ApSession {
    fn from(session: RemoteEncryptedSession) -> ApSession {
        ApSession {
            id: Option::from(session.ap_id),
            reference: session.reference,
            to: session.ap_to,
            attributed_to: session.attributed_to,
            instrument: serde_json::from_value(session.instrument).unwrap(),

            ..Default::default()
        }
    }
}
