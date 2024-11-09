use crate::db::Db;
use crate::schema::olm_one_time_keys;
use anyhow::Result;
use diesel::prelude::*;
use diesel::Insertable;
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::olm_one_time_keys::OlmOneTimeKey;
        pub use crate::models::pg::olm_one_time_keys::create_olm_one_time_key;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::olm_one_time_keys::OlmOneTimeKey;
        pub use crate::models::sqlite::olm_one_time_keys::create_olm_one_time_key;
        pub use crate::models::sqlite::olm_one_time_keys::get_olm_one_time_key_by_profile_id;
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = olm_one_time_keys)]
pub struct NewOlmOneTimeKey {
    pub profile_id: i32,
    pub uuid: String,
    pub olm_id: i32,
    pub key_data: String,
    pub distributed: bool,
    pub assignee: Option<String>,
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
            assignee: None,
        }
    }
}

pub async fn get_olm_one_time_keys_by_profile_id(
    conn: &Db,
    id: i32,
    limit: i64,
    offset: i64,
) -> Vec<OlmOneTimeKey> {
    conn.run(move |c| {
        let query = olm_one_time_keys::table
            .filter(olm_one_time_keys::profile_id.eq(id))
            .order(olm_one_time_keys::created_at.desc())
            .limit(limit)
            .offset(offset)
            .into_boxed();

        query.get_results::<OlmOneTimeKey>(c)
    })
    .await
    .unwrap_or(vec![])
}

pub async fn get_next_otk_by_profile_id(
    conn: &Db,
    actor_as_id: String,
    id: i32,
) -> Result<OlmOneTimeKey> {
    conn.run(move |c| {
        olm_one_time_keys::table
            .filter(
                olm_one_time_keys::profile_id
                    .eq(id)
                    .and(olm_one_time_keys::distributed.eq(false)),
            )
            .order(olm_one_time_keys::created_at.asc())
            .first::<OlmOneTimeKey>(c)
            .and_then(|otk| {
                diesel::update(olm_one_time_keys::table.find(otk.id))
                    .set((
                        olm_one_time_keys::distributed.eq(true),
                        olm_one_time_keys::assignee.eq(actor_as_id),
                    ))
                    .get_result::<OlmOneTimeKey>(c)
            })
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn get_otk_count_by_profile_id(conn: &Db, id: i32) -> Result<i64> {
    conn.run(move |c| {
        olm_one_time_keys::table
            .filter(
                olm_one_time_keys::profile_id
                    .eq(id)
                    .and(olm_one_time_keys::distributed.eq(false)),
            )
            .count()
            .get_result(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}
