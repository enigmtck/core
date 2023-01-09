use crate::{activity_pub::ApEncryptedMessage, schema::encrypted_messages};
use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[table_name = "encrypted_messages"]
pub struct NewEncryptedMessage {
    pub uuid: String,
    pub profile_id: i32,
    pub ap_to: Value,
    pub attributed_to: String,
    pub cc: Option<Value>,
    pub in_reply_to: Option<String>,
    pub encrypted_content: String,
}

impl From<ApEncryptedMessage> for NewEncryptedMessage {
    fn from(message: ApEncryptedMessage) -> Self {
        NewEncryptedMessage {
            ap_to: serde_json::to_value(&message.to).unwrap(),
            attributed_to: message.attributed_to,
            uuid: uuid::Uuid::new_v4().to_string(),
            encrypted_content: message.encrypted_content,
            in_reply_to: message.in_reply_to,
            ..Default::default()
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "encrypted_messages"]
pub struct EncryptedMessage {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub profile_id: i32,
    pub ap_to: Value,
    pub attributed_to: String,
    pub cc: Option<Value>,
    pub in_reply_to: Option<String>,
    pub encrypted_content: String,
}
