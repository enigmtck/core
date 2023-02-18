use crate::db::Db;
use crate::helper::get_local_username_from_ap_id;
use crate::schema::{self, profiles};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Insertable, Default)]
#[table_name = "profiles"]
pub struct NewProfile {
    pub uuid: String,
    pub username: String,
    pub display_name: String,
    pub summary: Option<String>,
    pub public_key: String,
    pub private_key: String,
    pub password: Option<String>,
    pub client_public_key: Option<String>,
    pub salt: Option<String>,
    pub client_private_key: Option<String>,
    pub olm_pickled_account: Option<String>,
    pub olm_pickled_account_hash: Option<String>,
    pub olm_identity_key: Option<String>,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Debug, Default)]
#[table_name = "profiles"]
pub struct Profile {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub username: String,
    pub display_name: String,
    pub summary: Option<String>,
    pub public_key: String,
    #[serde(skip_serializing)]
    pub private_key: String,
    #[serde(skip_serializing)]
    pub password: Option<String>,
    pub client_public_key: Option<String>,
    pub avatar_filename: String,
    pub banner_filename: Option<String>,
    pub salt: Option<String>,
    pub client_private_key: Option<String>,
    pub olm_pickled_account: Option<String>,
    pub olm_pickled_account_hash: Option<String>,
    pub olm_identity_key: Option<String>,
}

pub async fn create_profile(conn: &Db, profile: NewProfile) -> Option<Profile> {
    match conn
        .run(move |c| {
            diesel::insert_into(profiles::table)
                .values(&profile)
                .get_result::<Profile>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(e) => {
            log::debug!("database failure: {:#?}", e);
            Option::None
        }
    }
}

pub async fn get_profile(conn: &Db, id: i32) -> Option<Profile> {
    match conn
        .run(move |c| profiles::table.find(id).first::<Profile>(c))
        .await
    {
        Ok(x) => Option::from(x),
        Err(_) => Option::None,
    }
}

pub async fn get_profile_by_username(conn: &Db, username: String) -> Option<Profile> {
    match conn
        .run(move |c| {
            profiles::table
                .filter(profiles::username.eq(username))
                .first::<Profile>(c)
        })
        .await
    {
        Ok(x) => Option::from(x),
        Err(_) => Option::None,
    }
}

pub async fn get_profile_by_ap_id(conn: &Db, ap_id: String) -> Option<Profile> {
    if let Some(username) = get_local_username_from_ap_id(ap_id) {
        get_profile_by_username(conn, username).await
    } else {
        Option::None
    }
}

pub async fn update_olm_account_by_username(
    conn: &Db,
    username: String,
    account: String,
    account_hash: String,
) -> Option<Profile> {
    use schema::profiles::dsl::{
        olm_pickled_account, olm_pickled_account_hash, profiles, username as u,
    };

    match conn
        .run(move |c| {
            diesel::update(profiles.filter(u.eq(username)))
                .set((
                    olm_pickled_account.eq(account),
                    olm_pickled_account_hash.eq(account_hash),
                ))
                .get_result::<Profile>(c)
                .optional()
        })
        .await
    {
        Ok(x) => x,
        Err(_) => Option::None,
    }
}
