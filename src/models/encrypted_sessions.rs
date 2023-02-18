use crate::activity_pub::ApSession;
use crate::db::Db;
use crate::schema::{encrypted_sessions, olm_sessions};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::olm_sessions::OlmSession;

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

type IdentifiedEncryptedSession = (ApSession, i32);
impl From<IdentifiedEncryptedSession> for NewEncryptedSession {
    fn from(session: IdentifiedEncryptedSession) -> NewEncryptedSession {
        NewEncryptedSession {
            ap_to: session.0.to,
            attributed_to: session.0.attributed_to,
            reference: session.0.reference,
            instrument: serde_json::to_value(session.0.instrument).unwrap(),
            uuid: Uuid::new_v4().to_string(),
            profile_id: session.1,
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

pub async fn create_encrypted_session(
    conn: &Db,
    encrypted_session: NewEncryptedSession,
) -> Option<EncryptedSession> {
    match conn
        .run(move |c| {
            diesel::insert_into(encrypted_sessions::table)
                .values(&encrypted_session)
                .get_result::<EncryptedSession>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(e) => {
            log::debug!("{:#?}", e);
            Option::None
        }
    }
}

pub async fn get_encrypted_sessions_by_profile_id(
    conn: &Db,
    id: i32,
) -> Vec<(EncryptedSession, Option<OlmSession>)> {
    match conn
        .run(move |c| {
            encrypted_sessions::table
                .left_join(
                    olm_sessions::table
                        .on(encrypted_sessions::id.eq(olm_sessions::encrypted_session_id)),
                )
                .filter(encrypted_sessions::profile_id.eq(id))
                .get_results::<(EncryptedSession, Option<OlmSession>)>(c)
        })
        .await
    {
        Ok(x) => x,
        Err(_) => vec![],
    }
}

pub async fn get_encrypted_session_by_profile_id_and_ap_to(
    conn: &Db,
    id: i32,
    ap_to: String,
) -> Option<(EncryptedSession, Option<OlmSession>)> {
    match conn
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
                .optional()
        })
        .await
    {
        Ok(x) => x,
        Err(_) => None,
    }
}

pub async fn get_encrypted_session_by_uuid(
    conn: &Db,
    uuid: String,
) -> Option<(EncryptedSession, Option<OlmSession>)> {
    match conn
        .run(move |c| {
            encrypted_sessions::table
                .left_join(
                    olm_sessions::table
                        .on(encrypted_sessions::id.eq(olm_sessions::encrypted_session_id)),
                )
                .filter(encrypted_sessions::uuid.eq(uuid))
                .order_by(encrypted_sessions::updated_at.desc())
                .first::<(EncryptedSession, Option<OlmSession>)>(c)
                .optional()
        })
        .await
    {
        Ok(x) => x,
        Err(_) => None,
    }
}
