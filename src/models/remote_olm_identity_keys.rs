use crate::db::Db;
use crate::schema::remote_olm_identity_keys;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[table_name = "remote_olm_identity_keys"]
pub struct NewRemoteOlmIdentityKey {
    pub attributed_to: String,
    pub ap_id: String,
    pub key_data: String,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "remote_olm_identity_keys"]
pub struct RemoteOlmIdentityKey {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attributed_to: String,
    pub ap_id: String,
    pub key_data: String,
}

pub async fn create_remote_olm_identity_key(
    conn: &Db,
    remote_olm_identity_key: NewRemoteOlmIdentityKey,
) -> Option<RemoteOlmIdentityKey> {
    match conn
        .run(move |c| {
            diesel::insert_into(remote_olm_identity_keys::table)
                .values(&remote_olm_identity_key)
                .get_result::<RemoteOlmIdentityKey>(c)
                .optional()
        })
        .await
    {
        Ok(x) => x,
        Err(_) => Option::None,
    }
}

pub async fn get_remote_olm_identity_key_by_ap_id(
    conn: &Db,
    ap_id: String,
) -> Option<RemoteOlmIdentityKey> {
    match conn
        .run(move |c| {
            let query =
                remote_olm_identity_keys::table.filter(remote_olm_identity_keys::ap_id.eq(ap_id));

            query.first::<RemoteOlmIdentityKey>(c).optional()
        })
        .await
    {
        Ok(x) => x,
        Err(_) => Option::None,
    }
}
