use crate::db::Db;
use crate::schema::olm_sessions;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[table_name = "olm_sessions"]
pub struct NewOlmSession {
    pub profile_id: i32,
    pub remote_id: String,
    pub session_data: String,
    pub session_hash: String,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "olm_sessions"]
pub struct OlmSession {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub profile_id: i32,
    pub remote_id: String,
    pub session_data: String,
    pub session_hash: String,
}

pub async fn create_olm_session(conn: &Db, olm_session: NewOlmSession) -> Option<OlmSession> {
    match conn
        .run(move |c| {
            diesel::insert_into(olm_sessions::table)
                .values(&olm_session)
                .get_result::<OlmSession>(c)
                .optional()
        })
        .await
    {
        Ok(x) => x,
        Err(_) => Option::None,
    }
}

pub async fn get_olm_sessions_by_profile_id(
    conn: &Db,
    id: i32,
    limit: i64,
    offset: i64,
) -> Vec<OlmSession> {
    match conn
        .run(move |c| {
            let query = olm_sessions::table
                .filter(olm_sessions::profile_id.eq(id))
                .order(olm_sessions::created_at.desc())
                .limit(limit)
                .offset(offset)
                .into_boxed();

            query.get_results::<OlmSession>(c)
        })
        .await
    {
        Ok(x) => x,
        Err(_) => vec![],
    }
}

pub async fn get_olm_sessions_by_profile_id_and_ap_id(
    conn: &Db,
    profile_id: i32,
    ap_id: String,
) -> Vec<OlmSession> {
    match conn
        .run(move |c| {
            let query = olm_sessions::table
                .filter(olm_sessions::profile_id.eq(profile_id))
                .filter(olm_sessions::remote_id.eq(ap_id));

            query.get_results::<OlmSession>(c)
        })
        .await
    {
        Ok(x) => x,
        Err(_) => vec![],
    }
}
