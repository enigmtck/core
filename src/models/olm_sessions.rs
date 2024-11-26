use super::actors::Actor;
use crate::activity_pub::ApInstrument;
use crate::db::Db;
use crate::schema::olm_sessions;
use anyhow::{anyhow, Result};
use diesel::prelude::*;
use diesel::{AsChangeset, Insertable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::olm_sessions::OlmSession;
        pub use crate::models::pg::olm_sessions::create_or_update_olm_session;
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
    pub ap_conversation: String,
    pub owner_id: i32,
}

pub struct OlmSessionParams {
    pub uuid: Option<String>,
    pub instrument: ApInstrument,
    pub owner: Actor,
}

impl TryFrom<OlmSessionParams> for NewOlmSession {
    type Error = anyhow::Error;

    fn try_from(
        OlmSessionParams {
            uuid,
            instrument,
            owner,
        }: OlmSessionParams,
    ) -> Result<Self, Self::Error> {
        if !instrument.is_olm_session() {
            return Err(anyhow!("Instrument must be an OlmSession"));
        }

        let uuid = uuid.unwrap_or(Uuid::new_v4().to_string());
        let session_data = instrument.content.unwrap();
        let session_hash = instrument.hash.unwrap();
        let owner_as_id = owner.as_id;
        let owner_id = owner.id;
        let ap_conversation = instrument
            .conversation
            .ok_or_else(|| anyhow!("OlmSession must have a Conversation"))?;

        Ok(NewOlmSession {
            uuid,
            session_data,
            session_hash,
            owner_as_id,
            ap_conversation,
            owner_id,
        })
    }
}

pub async fn get_olm_session_by_conversation_and_actor(
    conn: &Db,
    conversation_as_id: String,
    actor_id: i32,
) -> Result<OlmSession> {
    conn.run(move |c| {
        olm_sessions::table
            .filter(
                olm_sessions::ap_conversation
                    .eq(conversation_as_id)
                    .and(olm_sessions::owner_id.eq(actor_id)),
            )
            .order(olm_sessions::updated_at.desc())
            .first::<OlmSession>(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}
