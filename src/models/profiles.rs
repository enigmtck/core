use crate::db::Db;
use crate::helper::{get_local_identifier, LocalIdentifierType};
use crate::schema::{self, profiles};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Insertable, Default)]
#[diesel(table_name = profiles)]
pub struct NewProfile {
    pub uuid: String,
    pub username: String,
    pub display_name: String,
    pub summary: Option<String>,
    pub summary_markdown: Option<String>,
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
#[diesel(table_name = profiles)]
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
    pub summary_markdown: Option<String>,
}

pub async fn create_profile(conn: &Db, profile: NewProfile) -> Option<Profile> {
    conn.run(move |c| {
        diesel::insert_into(profiles::table)
            .values(&profile)
            .get_result::<Profile>(c)
    })
    .await
    .ok()
}

pub async fn get_profile(conn: &Db, id: i32) -> Option<Profile> {
    conn.run(move |c| profiles::table.find(id).first::<Profile>(c))
        .await
        .ok()
}

pub async fn get_profile_by_username(conn: &Db, username: String) -> Option<Profile> {
    conn.run(move |c| {
        profiles::table
            .filter(profiles::username.eq(username))
            .first::<Profile>(c)
    })
    .await
    .ok()
}

pub async fn get_profile_by_ap_id(conn: &Db, ap_id: String) -> Option<Profile> {
    if let Some(x) = get_local_identifier(ap_id) {
        if x.kind == LocalIdentifierType::User {
            get_profile_by_username(conn, x.identifier).await
        } else {
            Option::None
        }
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

    conn.run(move |c| {
        diesel::update(profiles.filter(u.eq(username)))
            .set((
                olm_pickled_account.eq(account),
                olm_pickled_account_hash.eq(account_hash),
            ))
            .get_result::<Profile>(c)
    })
    .await
    .ok()
}

pub async fn update_avatar_by_username(
    conn: &Db,
    username: String,
    filename: String,
) -> Option<Profile> {
    conn.run(move |c| {
        diesel::update(profiles::table.filter(profiles::username.eq(username)))
            .set(profiles::avatar_filename.eq(filename))
            .get_result::<Profile>(c)
    })
    .await
    .ok()
}

pub async fn update_banner_by_username(
    conn: &Db,
    username: String,
    filename: String,
) -> Option<Profile> {
    conn.run(move |c| {
        diesel::update(profiles::table.filter(profiles::username.eq(username)))
            .set(profiles::banner_filename.eq(filename))
            .get_result::<Profile>(c)
    })
    .await
    .ok()
}

pub async fn update_summary_by_username(
    conn: &Db,
    username: String,
    summary: String,
    summary_markdown: String,
) -> Option<Profile> {
    conn.run(move |c| {
        diesel::update(profiles::table.filter(profiles::username.eq(username)))
            .set((
                profiles::summary.eq(summary),
                profiles::summary_markdown.eq(summary_markdown),
            ))
            .get_result::<Profile>(c)
    })
    .await
    .ok()
}
