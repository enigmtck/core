use crate::activity_pub::ApInstrument;
use crate::db::Db;
use crate::schema::olm_sessions;
use anyhow::anyhow;
use diesel::prelude::*;
use diesel::{AsChangeset, Insertable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::olm_sessions::OlmSession;
        pub use crate::models::pg::olm_sessions::create_or_update_olm_session;
        pub use crate::models::pg::olm_sessions::update_olm_session;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::olm_sessions::OlmSession;
        pub use crate::models::sqlite::olm_sessions::create_olm_session;
        pub use crate::models::sqlite::olm_sessions::update_olm_session_by_encrypted_session_id;
        pub use crate::models::sqlite::olm_sessions::update_olm_session;
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone, AsChangeset)]
#[diesel(table_name = olm_sessions)]
pub struct NewOlmSession {
    pub uuid: String,
    pub session_data: String,
    pub session_hash: String,
    pub owner_as_id: String,
    pub remote_as_id: String,
}

type OlmSessionParams = (ApInstrument, String, String);
impl TryFrom<OlmSessionParams> for NewOlmSession {
    type Error = anyhow::Error;

    fn try_from(
        (instrument, owner_as_id, remote_as_id): OlmSessionParams,
    ) -> Result<Self, Self::Error> {
        if !instrument.is_olm_session() {
            return Err(anyhow!("Instrument must be an OlmSession"));
        }

        let uuid = Uuid::new_v4().to_string();
        let session_data = instrument.content.unwrap();
        let session_hash = instrument.hash.unwrap();
        Ok(NewOlmSession {
            uuid,
            session_data,
            session_hash,
            owner_as_id,
            remote_as_id,
        })
    }
}

pub async fn get_olm_session_by_uuid(conn: &Db, uuid: String) -> Option<OlmSession> {
    conn.run(move |c| {
        olm_sessions::table
            .filter(olm_sessions::uuid.eq(uuid))
            .order(olm_sessions::updated_at.desc())
            .first::<OlmSession>(c)
    })
    .await
    .ok()
}
