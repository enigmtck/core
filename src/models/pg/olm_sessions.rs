use crate::db::Db;
use crate::models::olm_sessions::NewOlmSession;
use crate::schema::olm_sessions;
use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel::{AsChangeset, Identifiable, Queryable, QueryableByName, Selectable};
use rocket_sync_db_pools::diesel;
use serde::Serialize;

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

pub async fn create_or_update_olm_session(
    conn: &Db,
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

// pub async fn create_or_update_olm_session(
//     conn: &Db,
//     olm_session: NewOlmSession,
//     mutation_of: Option<String>,
// ) -> Option<OlmSession> {
//     conn.run(move |c| {
//         let query = sql_query(
//             if mutation_of.is_some() {
//                     "INSERT INTO olm_sessions (uuid, session_data, session_hash, owner_as_id, ap_conversation, owner_id)
//                      VALUES ($1, $2, $3, $4, $5, $6)
//                      ON CONFLICT (uuid) DO UPDATE SET
//                      session_data = $2,
//                      session_hash = $3,
//                      owner_as_id = $4,
//                      ap_conversation = $5
//                      WHERE olm_sessions.owner_id = excluded.owner_id AND excluded.session_hash = $7
//                      RETURNING *"
//             } else {
//                     "INSERT INTO olm_sessions (uuid, session_data, session_hash, owner_as_id, ap_conversation, owner_id)
//                      VALUES ($1, $2, $3, $4, $5, $6)
//                      ON CONFLICT (uuid) DO UPDATE SET
//                      session_data = $2,
//                      session_hash = $3,
//                      owner_as_id = $4,
//                      ap_conversation = $5
//                      WHERE olm_sessions.owner_id = excluded.owner_id
//                      RETURNING *"
//             }
//         );

//         let result = match mutation_of {
//             Some(mutation) => query
//                 .bind::<diesel::sql_types::Text, _>(olm_session.uuid)
//                 .bind::<diesel::sql_types::Text, _>(olm_session.session_data)
//                 .bind::<diesel::sql_types::Text, _>(olm_session.session_hash)
//                 .bind::<diesel::sql_types::Text, _>(olm_session.owner_as_id)
//                 .bind::<diesel::sql_types::Text, _>(olm_session.ap_conversation)
//                 .bind::<diesel::sql_types::Integer, _>(olm_session.owner_id)
//                 .bind::<diesel::sql_types::Text, _>(mutation)
//                 .get_result::<OlmSession>(c),
//             None => query
//                 .bind::<diesel::sql_types::Text, _>(olm_session.uuid)
//                 .bind::<diesel::sql_types::Text, _>(olm_session.session_data)
//                 .bind::<diesel::sql_types::Text, _>(olm_session.session_hash)
//                 .bind::<diesel::sql_types::Text, _>(olm_session.owner_as_id)
//                 .bind::<diesel::sql_types::Text, _>(olm_session.ap_conversation)
//                 .bind::<diesel::sql_types::Integer, _>(olm_session.owner_id)
//                 .get_result::<OlmSession>(c),
//         };

//         result.ok()
//     })
//     .await
// }
