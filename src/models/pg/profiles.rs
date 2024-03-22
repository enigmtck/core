use crate::db::Db;
use crate::schema::{self, profiles};
use crate::POOL;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::Serialize;

use crate::models::profiles::NewProfile;

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
    pub avatar_filename: Option<String>,
    pub banner_filename: Option<String>,
    pub salt: Option<String>,
    pub client_private_key: Option<String>,
    pub olm_pickled_account: Option<String>,
    pub olm_pickled_account_hash: Option<String>,
    pub olm_identity_key: Option<String>,
    pub summary_markdown: Option<String>,
}

impl Profile {
    pub fn set_avatar(mut self) -> Self {
        self.avatar_filename = Some(
            self.avatar_filename
                .unwrap_or((*crate::DEFAULT_AVATAR).clone()),
        );

        self.clone()
    }
}

pub async fn create_profile(conn: Option<&Db>, profile: NewProfile) -> Option<Profile> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(profiles::table)
                    .values(&profile)
                    .get_result::<Profile>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(profiles::table)
                .values(&profile)
                .get_result::<Profile>(&mut pool)
                .ok()
        }
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

pub async fn update_password_by_username(
    conn: &Db,
    username: String,
    password: String,
    client_private_key: String,
    olm_pickled_account: String,
) -> Option<Profile> {
    conn.run(move |c| {
        diesel::update(profiles::table.filter(profiles::username.eq(username)))
            .set((
                profiles::password.eq(password),
                profiles::client_private_key.eq(client_private_key),
                profiles::olm_pickled_account.eq(olm_pickled_account),
            ))
            .get_result::<Profile>(c)
    })
    .await
    .ok()
}
