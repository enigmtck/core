use crate::activity_pub::ApInstrument;
use crate::db::Db;
use crate::schema::{encrypted_sessions, olm_sessions};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

use super::encrypted_sessions::EncryptedSession;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[table_name = "olm_sessions"]
pub struct NewOlmSession {
    pub uuid: String,
    pub session_data: String,
    pub session_hash: String,
    pub encrypted_session_id: i32,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "olm_sessions"]
pub struct OlmSession {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub session_data: String,
    pub session_hash: String,
    pub encrypted_session_id: i32,
}

type LinkedApInstrument = (ApInstrument, i32);

impl From<LinkedApInstrument> for NewOlmSession {
    fn from((instrument, encrypted_session_id): LinkedApInstrument) -> Self {
        NewOlmSession {
            uuid: uuid::Uuid::new_v4().to_string(),
            session_data: instrument.content.unwrap_or_default(),
            session_hash: instrument.hash.unwrap_or_default(),
            encrypted_session_id,
        }
    }
}

pub async fn create_olm_session(conn: &Db, olm_session: NewOlmSession) -> Option<OlmSession> {
    conn.run(move |c| {
        diesel::insert_into(olm_sessions::table)
            .values(&olm_session)
            .get_result::<OlmSession>(c)
    })
    .await
    .ok()
}

pub async fn get_olm_session_by_uuid(
    conn: &Db,
    uuid: String,
) -> Option<(OlmSession, Option<EncryptedSession>)> {
    conn.run(move |c| {
        olm_sessions::table
            .left_join(
                encrypted_sessions::table
                    .on(olm_sessions::encrypted_session_id.eq(encrypted_sessions::id)),
            )
            .filter(olm_sessions::uuid.eq(uuid))
            .order(olm_sessions::updated_at.desc())
            .first::<(OlmSession, Option<EncryptedSession>)>(c)
    })
    .await
    .ok()
}

pub async fn get_olm_session_by_encrypted_session_id(
    conn: &Db,
    encrypted_session_id: i32,
) -> Option<OlmSession> {
    conn.run(move |c| {
        let query = olm_sessions::table
            .filter(olm_sessions::encrypted_session_id.eq(encrypted_session_id))
            .order(olm_sessions::updated_at.desc());

        query.first::<OlmSession>(c)
    })
    .await
    .ok()
}

pub async fn update_olm_session_by_encrypted_session_id(
    conn: &Db,
    encrypted_session_id: i32,
    session_data: String,
    session_hash: String,
) -> Option<OlmSession> {
    log::debug!("UPDATING OLM SESSION\nencrypted_session_id: {encrypted_session_id}\nsession_data: {session_data}\nsession_hash: {session_hash}");

    conn.run(move |c| {
        diesel::update(
            olm_sessions::table.filter(olm_sessions::encrypted_session_id.eq(encrypted_session_id)),
        )
        .set((
            olm_sessions::session_data.eq(session_data),
            olm_sessions::session_hash.eq(session_hash),
        ))
        .get_result::<OlmSession>(c)
    })
    .await
    .ok()
}
