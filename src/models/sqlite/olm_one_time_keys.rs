use crate::db::Db;
use crate::models::olm_one_time_keys::NewOlmOneTimeKey;
use crate::schema::olm_one_time_keys;
use crate::POOL;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::Serialize;

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = olm_one_time_keys)]
pub struct OlmOneTimeKey {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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
    conn.run(move |c| {
        diesel::insert_into(olm_one_time_keys::table)
            .values(&olm_one_time_key)
            .execute(c)
    })
    .await
    .ok()?;

    conn.run(move |c| {
        olm_one_time_keys::table
            .order(olm_one_time_keys::id.desc())
            .first::<OlmOneTimeKey>(c)
    })
    .await
    .ok()
}

pub async fn get_olm_one_time_key_by_profile_id(
    conn: Option<&Db>,
    profile_id: i32,
) -> Option<OlmOneTimeKey> {
    match conn {
        Some(conn) => {
            let otk = conn
                .run(move |c| {
                    olm_one_time_keys::table
                        .filter(olm_one_time_keys::profile_id.eq(profile_id))
                        .filter(olm_one_time_keys::distributed.eq(false))
                        .order(olm_one_time_keys::created_at.asc())
                        .first::<OlmOneTimeKey>(c)
                })
                .await
                .ok()?;

            conn.run(move |c| {
                diesel::update(olm_one_time_keys::table.find(otk.id))
                    .set(olm_one_time_keys::distributed.eq(true))
                    .execute(c)
            })
            .await
            .ok()?;

            Some(otk)
        }
        None => {
            let mut pool = POOL.get().ok()?;
            let otk = olm_one_time_keys::table
                .filter(olm_one_time_keys::profile_id.eq(profile_id))
                .filter(olm_one_time_keys::distributed.eq(false))
                .order(olm_one_time_keys::created_at.asc())
                .first::<OlmOneTimeKey>(&mut pool)
                .ok()?;

            diesel::update(olm_one_time_keys::table.find(otk.id))
                .set(olm_one_time_keys::distributed.eq(true))
                .execute(&mut pool)
                .ok()?;

            Some(otk)
        }
    }
}
