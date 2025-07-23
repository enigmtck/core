use super::actors::Actor;
use crate::db::runner::DbRunner;
use crate::helper::get_session_as_id_from_uuid;
use crate::schema::olm_sessions;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel::{AsChangeset, Identifiable, Queryable, QueryableByName, Selectable};
use jdt_activity_pub::{ApInstrument, ApInstrumentType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Identifiable,
    Queryable,
    QueryableByName,
    Selectable,
    AsChangeset,
    Serialize,
    Clone,
    Default,
    Debug,
)]
#[diesel(table_name = olm_sessions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OlmSession {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub session_data: String,
    pub session_hash: String,
    pub owner_as_id: String,
    pub ap_conversation: String,
    pub owner_id: i32,
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

impl From<OlmSession> for ApInstrument {
    fn from(session: OlmSession) -> Self {
        Self {
            kind: ApInstrumentType::OlmSession,
            id: Some(get_session_as_id_from_uuid(session.uuid.clone())),
            content: Some(session.session_data),
            hash: Some(session.session_hash),
            uuid: None,
            name: None,
            url: None,
            mutation_of: None,
            conversation: None,
            activity: None,
        }
    }
}

pub async fn create_or_update_olm_session<C: DbRunner>(
    conn: &C,
    olm_session: NewOlmSession,
    mutation_of: Option<String>,
) -> Result<OlmSession> {
    use diesel::query_dsl::methods::FilterDsl;
    conn.run(move |c| {
        let query = diesel::insert_into(olm_sessions::table)
            .values(&olm_session)
            .on_conflict((olm_sessions::ap_conversation, olm_sessions::owner_as_id))
            .do_update()
            .set(&olm_session);

        if let Some(mutation_of) = mutation_of {
            query
                .filter(excluded(olm_sessions::session_hash).eq(mutation_of))
                .get_result::<OlmSession>(c)
        } else {
            query.get_result::<OlmSession>(c)
        }
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn update_olm_session_by_encrypted_session_id<C: DbRunner>(
    conn: &C,
    _id: i32,
    session_data: String,
    session_hash: String,
) -> Option<OlmSession> {
    conn.run(move |c| {
        diesel::update(olm_sessions::table)
            .set((
                olm_sessions::session_data.eq(session_data),
                olm_sessions::session_hash.eq(session_hash),
            ))
            .get_result(c)
    })
    .await
    .ok()
}

pub async fn get_olm_session_by_conversation_and_actor<C: DbRunner>(
    conn: &C,
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
