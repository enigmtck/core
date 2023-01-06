use crate::activity_pub::{ApContext, ApInstrument, ApObjectType};
use serde::{Deserialize, Serialize};

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
            id: Option::None,
            to: String::new(),
            attributed_to: String::new(),
            instrument: ApInstrument::default(),
            reference: Option::None,
        }
    }
}
