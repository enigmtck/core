use crate::db::Db;
use crate::models::olm_one_time_keys::NewOlmOneTimeKey;
use crate::schema::olm_one_time_keys;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::Serialize;

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = olm_one_time_keys)]
pub struct OlmOneTimeKey {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub profile_id: i32,
    pub olm_id: i32,
    pub key_data: String,
    pub distributed: bool,
    pub assignee: Option<String>,
}

pub async fn create_olm_one_time_key(
    conn: &Db,
    olm_one_time_key: NewOlmOneTimeKey,
) -> Option<OlmOneTimeKey> {
    conn.run(move |c| {
        diesel::insert_into(olm_one_time_keys::table)
            .values(&olm_one_time_key)
            .get_result::<OlmOneTimeKey>(c)
    })
    .await
    .ok()
}
