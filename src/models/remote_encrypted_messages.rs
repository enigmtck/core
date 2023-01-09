use crate::activity_pub::{ApActivity, ApEncryptedMessage, ApObject};
use crate::schema::remote_encrypted_messages;
use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[table_name = "remote_encrypted_messages"]
pub struct NewRemoteEncryptedMessage {
    pub profile_id: i32,
    pub ap_id: String,
    pub ap_to: Value,
    pub cc: Option<Value>,
    pub attributed_to: String,
    pub published: String,
    pub in_reply_to: Option<String>,
    pub encrypted_content: Value,
}

impl From<ApEncryptedMessage> for NewRemoteEncryptedMessage {
    fn from(encrypted_message: ApEncryptedMessage) -> NewRemoteEncryptedMessage {
        NewRemoteEncryptedMessage {
            ap_id: encrypted_message.id,
            ap_to: serde_json::to_value(&encrypted_message.to).unwrap(),
            cc: Some(serde_json::to_value(&encrypted_message.cc).unwrap()),
            attributed_to: encrypted_message.attributed_to,
            published: encrypted_message.published,
            in_reply_to: encrypted_message.in_reply_to,
            encrypted_content: serde_json::to_value(&encrypted_message.encrypted_content).unwrap(),
            ..Default::default()
        }
    }
}

impl From<ApActivity> for NewRemoteEncryptedMessage {
    fn from(activity: ApActivity) -> NewRemoteEncryptedMessage {
        if let ApObject::EncryptedMessage(message) = activity.object {
            NewRemoteEncryptedMessage::from(message)
        } else {
            NewRemoteEncryptedMessage::default()
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "remote_encrypted_messages"]
pub struct RemoteEncryptedMessage {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile_id: i32,
    pub ap_id: String,
    pub ap_to: Value,
    pub cc: Option<Value>,
    pub attributed_to: String,
    pub published: String,
    pub in_reply_to: Option<String>,
    pub encrypted_content: Value,
}
