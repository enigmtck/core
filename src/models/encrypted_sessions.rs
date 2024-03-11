use crate::activity_pub::ApSession;
use crate::db::Db;
use crate::schema::{encrypted_sessions, olm_sessions};
use crate::POOL;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::olm_sessions::OlmSession;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = encrypted_sessions)]
pub struct NewEncryptedSession {
    pub profile_id: i32,
    pub ap_to: String,
    pub attributed_to: String,
    pub instrument: String,
    pub reference: Option<String>,
    pub uuid: String,
}

type IdentifiedEncryptedSession = (ApSession, i32);
impl From<IdentifiedEncryptedSession> for NewEncryptedSession {
    fn from((session, profile_id): IdentifiedEncryptedSession) -> NewEncryptedSession {
        NewEncryptedSession {
            ap_to: session.to.to_string(),
            attributed_to: session.attributed_to.to_string(),
            reference: session.reference,
            instrument: serde_json::to_string(&session.instrument).unwrap(),
            uuid: Uuid::new_v4().to_string(),
            profile_id,
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = encrypted_sessions)]
pub struct EncryptedSession {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub profile_id: i32,
    pub ap_to: String,
    pub attributed_to: String,
    pub instrument: String,
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

pub async fn get_encrypted_sessions_by_profile_id(
    conn: &Db,
    id: i32,
) -> Vec<(EncryptedSession, Option<OlmSession>)> {
    conn.run(move |c| {
        encrypted_sessions::table
            .left_join(
                olm_sessions::table
                    .on(encrypted_sessions::id.eq(olm_sessions::encrypted_session_id)),
            )
            .filter(encrypted_sessions::profile_id.eq(id))
            .get_results::<(EncryptedSession, Option<OlmSession>)>(c)
    })
    .await
    .unwrap_or(vec![])
}

pub async fn get_encrypted_session_by_profile_id_and_ap_to(
    conn: Option<&Db>,
    id: i32,
    ap_to: String,
) -> Option<(EncryptedSession, Option<OlmSession>)> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                encrypted_sessions::table
                    .left_join(
                        olm_sessions::table
                            .on(encrypted_sessions::id.eq(olm_sessions::encrypted_session_id)),
                    )
                    .filter(encrypted_sessions::profile_id.eq(id))
                    .filter(encrypted_sessions::ap_to.eq(ap_to))
                    .order_by(encrypted_sessions::updated_at.desc())
                    .first::<(EncryptedSession, Option<OlmSession>)>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            encrypted_sessions::table
                .left_join(
                    olm_sessions::table
                        .on(encrypted_sessions::id.eq(olm_sessions::encrypted_session_id)),
                )
                .filter(encrypted_sessions::profile_id.eq(id))
                .filter(encrypted_sessions::ap_to.eq(ap_to))
                .order_by(encrypted_sessions::updated_at.desc())
                .first::<(EncryptedSession, Option<OlmSession>)>(&mut pool)
                .ok()
        }
    }
}

pub async fn get_encrypted_session_by_uuid(
    conn: Option<&Db>,
    uuid: String,
) -> Option<(EncryptedSession, Option<OlmSession>)> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                encrypted_sessions::table
                    .left_join(
                        olm_sessions::table
                            .on(encrypted_sessions::id.eq(olm_sessions::encrypted_session_id)),
                    )
                    .filter(encrypted_sessions::uuid.eq(uuid))
                    .order_by(encrypted_sessions::updated_at.desc())
                    .first::<(EncryptedSession, Option<OlmSession>)>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            encrypted_sessions::table
                .left_join(
                    olm_sessions::table
                        .on(encrypted_sessions::id.eq(olm_sessions::encrypted_session_id)),
                )
                .filter(encrypted_sessions::uuid.eq(uuid))
                .order_by(encrypted_sessions::updated_at.desc())
                .first::<(EncryptedSession, Option<OlmSession>)>(&mut pool)
                .ok()
        }
    }
}

pub async fn get_encrypted_session_by_id_and_profile_id(
    conn: &Db,
    id: i32,
    profile_id: i32,
) -> Option<(EncryptedSession, Option<OlmSession>)> {
    conn.run(move |c| {
        encrypted_sessions::table
            .left_join(
                olm_sessions::table
                    .on(encrypted_sessions::id.eq(olm_sessions::encrypted_session_id)),
            )
            .filter(encrypted_sessions::id.eq(id))
            .filter(encrypted_sessions::profile_id.eq(profile_id))
            .order_by(encrypted_sessions::updated_at.desc())
            .first::<(EncryptedSession, Option<OlmSession>)>(c)
    })
    .await
    .ok()
}
