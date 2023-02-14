use crate::db::Db;
use crate::schema::remote_olm_one_time_keys;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[table_name = "remote_olm_one_time_keys"]
pub struct NewRemoteOlmOneTimeKey {
    pub attributed_to: String,
    pub ap_id: String,
    pub ap_to: String,
    pub olm_id: i32,
    pub key_data: String,
    pub consumed: bool,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "remote_olm_one_time_keys"]
pub struct RemoteOlmOneTimeKey {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attributed_to: String,
    pub ap_id: String,
    pub ap_to: String,
    pub olm_id: i32,
    pub key_data: String,
    pub consumed: bool,
}

pub async fn create_remote_olm_one_time_key(
    conn: &Db,
    remote_olm_one_time_key: NewRemoteOlmOneTimeKey,
) -> Option<RemoteOlmOneTimeKey> {
    match conn
        .run(move |c| {
            diesel::insert_into(remote_olm_one_time_keys::table)
                .values(&remote_olm_one_time_key)
                .get_result::<RemoteOlmOneTimeKey>(c)
                .optional()
        })
        .await
    {
        Ok(x) => x,
        Err(_) => Option::None,
    }
}

pub async fn get_remote_olm_one_time_key_by_ap_id(
    conn: &Db,
    ap_id: String,
) -> Option<RemoteOlmOneTimeKey> {
    match conn
        .run(move |c| {
            let query =
                remote_olm_one_time_keys::table.filter(remote_olm_one_time_keys::ap_id.eq(ap_id));

            query.first::<RemoteOlmOneTimeKey>(c).optional()
        })
        .await
    {
        Ok(x) => x,
        Err(_) => Option::None,
    }
}
