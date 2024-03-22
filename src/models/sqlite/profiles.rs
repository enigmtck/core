use crate::db::Db;
use crate::schema::profiles;
use crate::POOL;
use chrono::NaiveDateTime;
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(profiles::table)
                    .values(&profile)
                    .execute(c)
            })
            .await
            .ok()?;

            conn.run(move |c| {
                profiles::table
                    .order(profiles::id.desc())
                    .first::<Profile>(c)
            })
            .await
            .ok()
        }
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(profiles::table)
                .values(&profile)
                .execute(&mut pool)
                .ok()?;

            profiles::table
                .order(profiles::id.desc())
                .first::<Profile>(&mut pool)
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
    conn.run(move |c| {
        diesel::update(profiles::table.filter(profiles::username.eq(&username.clone())))
            .set((
                profiles::olm_pickled_account.eq(account),
                profiles::olm_pickled_account_hash.eq(account_hash),
            ))
            .execute(c)?;

        profiles::table
            .filter(profiles::username.eq(username))
            .first::<Profile>(c)
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
        diesel::update(profiles::table.filter(profiles::username.eq(username.clone())))
            .set(profiles::avatar_filename.eq(filename))
            .execute(c)?;

        profiles::table
            .filter(profiles::username.eq(username))
            .first::<Profile>(c)
    })
    .await
    .ok()
}

pub async fn update_banner_by_username(
    conn: &Db,
    username: String,
    filename: String,
) -> Option<Profile> {
    let username_clone = username.clone();

    conn.run(move |c| {
        diesel::update(profiles::table.filter(profiles::username.eq(username_clone)))
            .set(profiles::banner_filename.eq(filename))
            .execute(c)?;

        profiles::table
            .filter(profiles::username.eq(username))
            .first::<Profile>(c)
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
        diesel::update(profiles::table.filter(profiles::username.eq(username.clone())))
            .set((
                profiles::summary.eq(summary),
                profiles::summary_markdown.eq(summary_markdown),
            ))
            .execute(c)?;

        profiles::table
            .filter(profiles::username.eq(username))
            .first::<Profile>(c)
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
        diesel::update(profiles::table.filter(profiles::username.eq(username.clone())))
            .set((
                profiles::password.eq(password),
                profiles::client_private_key.eq(client_private_key),
                profiles::olm_pickled_account.eq(olm_pickled_account),
            ))
            .execute(c)?;

        profiles::table
            .filter(profiles::username.eq(username))
            .first::<Profile>(c)
    })
    .await
    .ok()
}
