use crate::activity_pub::ApInstrument;
use crate::db::Db;
use crate::models::encrypted_sessions::EncryptedSession;
use crate::schema::{encrypted_sessions, olm_sessions};
use diesel::prelude::*;
use diesel::Insertable;
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::olm_sessions::OlmSession;
        pub use crate::models::pg::olm_sessions::create_olm_session;
        pub use crate::models::pg::olm_sessions::update_olm_session_by_encrypted_session_id;
        pub use crate::models::pg::olm_sessions::update_olm_session;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::olm_sessions::OlmSession;
        pub use crate::models::sqlite::olm_sessions::create_olm_session;
        pub use crate::models::sqlite::olm_sessions::update_olm_session_by_encrypted_session_id;
        pub use crate::models::sqlite::olm_sessions::update_olm_session;
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = olm_sessions)]
pub struct NewOlmSession {
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
