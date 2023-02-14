use crate::db::Db;
use crate::schema::olm_one_time_keys;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[table_name = "olm_one_time_keys"]
pub struct NewOlmOneTimeKey {
    pub profile_id: i32,
    pub uuid: String,
    pub olm_id: i32,
    pub key_data: String,
    pub distributed: bool,
}

// profile_id, olm_id, key_data
type KeyTuple = (i32, String, String);

impl From<KeyTuple> for NewOlmOneTimeKey {
    fn from((profile_id, olm_id, key_data): KeyTuple) -> NewOlmOneTimeKey {
        NewOlmOneTimeKey {
            profile_id,
            uuid: uuid::Uuid::new_v4().to_string(),
            olm_id: olm_id.parse::<i32>().expect("INVALID FORMAT FOR OLM_ID"),
            key_data,
            distributed: false,
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "olm_one_time_keys"]
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
}

pub async fn create_olm_one_time_key(
    conn: &Db,
    olm_one_time_key: NewOlmOneTimeKey,
) -> Option<OlmOneTimeKey> {
    match conn
        .run(move |c| {
            diesel::insert_into(olm_one_time_keys::table)
                .values(&olm_one_time_key)
                .get_result::<OlmOneTimeKey>(c)
                .optional()
        })
        .await
    {
        Ok(x) => x,
        Err(_) => Option::None,
    }
}

pub async fn get_olm_one_time_keys_by_profile_id(
    conn: &Db,
    id: i32,
    limit: i64,
    offset: i64,
) -> Vec<OlmOneTimeKey> {
    match conn
        .run(move |c| {
            let query = olm_one_time_keys::table
                .filter(olm_one_time_keys::profile_id.eq(id))
                .order(olm_one_time_keys::created_at.desc())
                .limit(limit)
                .offset(offset)
                .into_boxed();

            query.get_results::<OlmOneTimeKey>(c)
        })
        .await
    {
        Ok(x) => x,
        Err(_) => vec![],
    }
}

pub async fn get_olm_one_time_key_by_profile_id(
    conn: &Db,
    profile_id: i32,
) -> Option<OlmOneTimeKey> {
    match conn
        .run(move |c| {
            let query = olm_one_time_keys::table
                .filter(olm_one_time_keys::profile_id.eq(profile_id))
                .filter(olm_one_time_keys::distributed.eq(false))
                .order(olm_one_time_keys::created_at.asc());

            query.first::<OlmOneTimeKey>(c).optional()
        })
        .await
    {
        Ok(x) => x,
        Err(_) => Option::None,
    }
}
