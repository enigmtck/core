use crate::db::Db;
use crate::schema::encrypted_sessions;
use crate::POOL;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = encrypted_sessions)]
pub struct NewEncryptedSession {
    pub profile_id: i32,
    pub ap_to: String,
    pub attributed_to: String,
    pub instrument: Value,
    pub reference: Option<String>,
    pub uuid: String,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = encrypted_sessions)]
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

pub async fn create_encrypted_session(
    conn: Option<&Db>,
    encrypted_session: NewEncryptedSession,
) -> Option<EncryptedSession> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(encrypted_sessions::table)
                    .values(&encrypted_session)
                    .get_result::<EncryptedSession>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(encrypted_sessions::table)
                .values(&encrypted_session)
                .get_result::<EncryptedSession>(&mut pool)
                .ok()
        }
    }
}
