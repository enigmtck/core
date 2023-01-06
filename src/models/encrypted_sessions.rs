use crate::activity_pub::ApSession;
use crate::schema::encrypted_sessions;
use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[table_name = "encrypted_sessions"]
pub struct NewEncryptedSession {
    pub profile_id: i32,
    pub ap_to: String,
    pub attributed_to: String,
    pub instrument: Value,
    pub reference: Option<String>,
    pub uuid: String,
}

impl From<ApSession> for NewEncryptedSession {
    fn from(session: ApSession) -> NewEncryptedSession {
        NewEncryptedSession {
            ap_to: session.to,
            attributed_to: session.attributed_to,
            reference: session.reference,
            instrument: serde_json::to_value(session.instrument).unwrap(),
            uuid: Uuid::new_v4().to_string(),
            ..Default::default()
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "encrypted_sessions"]
pub struct EncryptedSession {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile_id: i32,
    pub ap_to: String,
    pub attributed_to: String,
    pub instrument: Value,
    pub reference: Option<String>,
    pub uuid: String,
}
