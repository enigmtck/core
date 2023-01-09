use crate::activity_pub::{ApContext, ApObjectType};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApEncryptedMessage {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    pub id: String,
    #[serde(rename = "type")]
    pub kind: ApObjectType,
    pub to: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<String>>,
    pub attributed_to: String,
    pub published: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<String>,
    pub encrypted_content: String,
}

impl Default for ApEncryptedMessage {
    fn default() -> ApEncryptedMessage {
        ApEncryptedMessage {
            context: Option::from(ApContext::Plain(
                "https://www.w3.org/ns/activitystreams".to_string(),
            )),
            id: format!(
                "https://{}/encrypted-messages/{}",
                *crate::SERVER_NAME,
                uuid::Uuid::new_v4()
            ),
            kind: ApObjectType::EncryptedMessage,
            to: vec![],
            cc: Option::None,
            attributed_to: String::new(),
            published: String::new(),
            in_reply_to: Option::None,
            encrypted_content: String::new(),
        }
    }
}
