use crate::activity_pub::{ApActivity, ApObject};
use crate::schema::remote_encrypted_sessions;
use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[table_name = "remote_encrypted_sessions"]
pub struct NewRemoteEncryptedSession {
    pub profile_id: i32,
    pub actor: String,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: String,
    pub attributed_to: String,
    pub instrument: Value,
    pub reference: Option<String>,
}

impl From<ApActivity> for NewRemoteEncryptedSession {
    fn from(activity: ApActivity) -> NewRemoteEncryptedSession {
        let mut ret = NewRemoteEncryptedSession::default();

        if let ApObject::Session(session) = activity.object {
            ret.actor = activity.actor;
            ret.kind = activity.kind.to_string();
            ret.ap_id = session.base.id.unwrap();
            ret.ap_to = session.to;
            ret.attributed_to = session.attributed_to;
            ret.reference = session.base.reference;
            ret.instrument = serde_json::to_value(session.instrument).unwrap();
        }

        ret
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "remote_encrypted_sessions"]
pub struct RemoteEncryptedSession {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile_id: i32,
    pub actor: String,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: String,
    pub attributed_to: String,
    pub instrument: Value,
    pub reference: Option<String>,
}
