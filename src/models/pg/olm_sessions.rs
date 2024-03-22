use crate::db::Db;
use crate::models::olm_sessions::NewOlmSession;
use crate::schema::olm_sessions;
use crate::POOL;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::Serialize;

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = olm_sessions)]
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

pub async fn create_olm_session(
    conn: Option<&Db>,
    olm_session: NewOlmSession,
) -> Option<OlmSession> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(olm_sessions::table)
                    .values(&olm_session)
                    .get_result::<OlmSession>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(olm_sessions::table)
                .values(&olm_session)
                .get_result::<OlmSession>(&mut pool)
                .ok()
        }
    }
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

pub async fn update_olm_session(
    conn: Option<&Db>,
    uuid: String,
    session_data: String,
    session_hash: String,
) -> Option<OlmSession> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::update(olm_sessions::table.filter(olm_sessions::uuid.eq(uuid)))
                    .set((
                        olm_sessions::session_data.eq(session_data),
                        olm_sessions::session_hash.eq(session_hash),
                    ))
                    .get_result::<OlmSession>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::update(olm_sessions::table.filter(olm_sessions::uuid.eq(uuid)))
                .set((
                    olm_sessions::session_data.eq(session_data),
                    olm_sessions::session_hash.eq(session_hash),
                ))
                .get_result::<OlmSession>(&mut pool)
                .ok()
        }
    }
}
