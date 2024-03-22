use crate::db::Db;
use crate::schema::remote_encrypted_sessions;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[diesel(table_name = remote_encrypted_sessions)]
pub struct NewRemoteEncryptedSession {
    pub profile_id: i32,
    pub actor: String,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: String,
    pub attributed_to: String,
    pub instrument: String,
    pub reference: Option<String>,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Default, Debug)]
#[diesel(table_name = remote_encrypted_sessions)]
pub struct RemoteEncryptedSession {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub profile_id: i32,
    pub actor: String,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: String,
    pub attributed_to: String,
    pub instrument: String,
    pub reference: Option<String>,
}

pub async fn create_remote_encrypted_session(
    conn: &Db,
    remote_encrypted_session: NewRemoteEncryptedSession,
) -> Option<RemoteEncryptedSession> {
    conn.run(move |c| {
        diesel::insert_into(remote_encrypted_sessions::table)
            .values(&remote_encrypted_session)
            .execute(c)
    })
    .await
    .ok()?;

    conn.run(move |c| {
        remote_encrypted_sessions::table
            .order(remote_encrypted_sessions::id.desc())
            .first::<RemoteEncryptedSession>(c)
    })
    .await
    .ok()
}
