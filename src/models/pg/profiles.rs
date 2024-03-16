use std::collections::HashSet;

use crate::activity_pub::ApAddress;
use crate::db::Db;
use crate::helper::{get_local_identifier, is_local, LocalIdentifierType};
use crate::schema::{self, profiles};
use crate::POOL;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

use super::followers::get_followers_by_profile_id;
use super::remote_actors::{get_remote_actor_by_ap_id, RemoteActor};

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

pub async fn get_profile(conn: Option<&Db>, id: i32) -> Option<Profile> {
    match conn {
        Some(conn) => conn
            .run(move |c| profiles::table.find(id).first::<Profile>(c))
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            profiles::table.find(id).first::<Profile>(&mut pool).ok()
        }
    }
}

pub async fn get_profile_by_username(conn: Option<&Db>, username: String) -> Option<Profile> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                profiles::table
                    .filter(profiles::username.eq(username))
                    .first::<Profile>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            profiles::table
                .filter(profiles::username.eq(username))
                .first::<Profile>(&mut pool)
                .ok()
        }
    }
}

pub async fn get_profile_by_uuid(conn: Option<&Db>, uuid: String) -> Option<Profile> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                profiles::table
                    .filter(profiles::uuid.eq(uuid))
                    .first::<Profile>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            profiles::table
                .filter(profiles::uuid.eq(uuid))
                .first::<Profile>(&mut pool)
                .ok()
        }
    }
}

pub async fn get_profile_by_ap_id(conn: Option<&Db>, ap_id: String) -> Option<Profile> {
    if let Some(x) = get_local_identifier(ap_id) {
        if x.kind == LocalIdentifierType::User {
            get_profile_by_username(conn, x.identifier).await
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn get_follower_inboxes(conn: Option<&Db>, profile: Profile) -> Vec<ApAddress> {
    let mut inboxes: HashSet<ApAddress> = HashSet::new();

    for (_follower, remote_actor) in get_followers_by_profile_id(conn, profile.id).await {
        if let Some(remote_actor) = remote_actor {
            inboxes.insert(ApAddress::Address(remote_actor.inbox));
        }
    }

    Vec::from_iter(inboxes)
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

pub async fn guaranteed_profile(conn: Option<&Db>, profile: Option<Profile>) -> Profile {
    match profile {
        Some(profile) => profile,
        None => get_profile_by_username(conn, (*crate::SYSTEM_USER).clone())
            .await
            .expect("unable to retrieve system user"),
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum ActorLike {
    RemoteActor(RemoteActor),
    Profile(Profile),
}

pub async fn get_actory(conn: &Db, id: String) -> Option<ActorLike> {
    if is_local(id.clone()) {
        let identifier = get_local_identifier(id.clone())?;
        if identifier.kind == LocalIdentifierType::User {
            let profile = get_profile_by_username(conn.into(), identifier.identifier).await?;
            Some(ActorLike::Profile(profile))
        } else {
            None
        }
    } else {
        let remote_actor = get_remote_actor_by_ap_id(conn.into(), id).await.ok()?;
        Some(ActorLike::RemoteActor(remote_actor))
    }
}
