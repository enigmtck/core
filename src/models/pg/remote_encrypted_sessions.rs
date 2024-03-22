use crate::db::Db;
use crate::schema::remote_encrypted_sessions;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[diesel(table_name = remote_encrypted_sessions)]
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

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Default, Debug)]
#[diesel(table_name = remote_encrypted_sessions)]
pub struct RemoteEncryptedSession {
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

pub async fn create_remote_encrypted_session(
    conn: &Db,
    remote_encrypted_session: NewRemoteEncryptedSession,
) -> Option<RemoteEncryptedSession> {
    if let Ok(x) = conn
        .run(move |c| {
            diesel::insert_into(remote_encrypted_sessions::table)
                .values(&remote_encrypted_session)
                .get_result::<RemoteEncryptedSession>(c)
        })
        .await
    {
        Some(x)
    } else {
        Option::None
    }
}
