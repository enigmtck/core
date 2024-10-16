use crate::db::Db;
use crate::models::to_serde;
use crate::schema::actors;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

//use super::coalesced_activity::CoalescedActivity;

#[derive(
    diesel_derive_enum::DbEnum, Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq,
)]
#[ExistingTypePath = "crate::schema::sql_types::ActorType"]
pub enum ActorType {
    #[default]
    Person,
    Service,
    Group,
    Organization,
    Application,
}

impl fmt::Display for ActorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ActorType {
    pub fn is_person(&self) -> bool {
        matches!(self, ActorType::Person)
    }

    pub fn is_service(&self) -> bool {
        matches!(self, ActorType::Service)
    }

    pub fn is_group(&self) -> bool {
        matches!(self, ActorType::Group)
    }

    pub fn is_organization(&self) -> bool {
        matches!(self, ActorType::Organization)
    }

    pub fn is_application(&self) -> bool {
        matches!(self, ActorType::Application)
    }
}

impl TryFrom<String> for ActorType {
    type Error = anyhow::Error;

    fn try_from(actor: String) -> Result<Self, Self::Error> {
        match actor.to_lowercase().as_str() {
            "person" => Ok(ActorType::Person),
            "service" => Ok(ActorType::Service),
            "group" => Ok(ActorType::Group),
            "organization" => Ok(ActorType::Organization),
            "application" => Ok(ActorType::Application),
            _ => Err(anyhow!("unimplemented ActorType")),
        }
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, AsChangeset)]
#[diesel(table_name = actors)]
pub struct NewActor {
    pub ek_uuid: Option<String>,
    pub ek_username: Option<String>,
    pub ek_summary_markdown: Option<String>,
    pub ek_avatar_filename: Option<String>,
    pub ek_banner_filename: Option<String>,
    pub ek_private_key: Option<String>,
    pub ek_password: Option<String>,
    pub ek_client_public_key: Option<String>,
    pub ek_client_private_key: Option<String>,
    pub ek_salt: Option<String>,
    pub ek_olm_pickled_account: Option<String>,
    pub ek_olm_pickled_account_hash: Option<String>,
    pub ek_olm_identity_key: Option<String>,
    pub ek_webfinger: Option<String>,
    pub ek_checked_at: DateTime<Utc>,
    pub ek_hashtags: Value,
    pub as_type: ActorType,
    pub as_context: Option<Value>,
    pub as_id: String,
    pub as_name: Option<String>,
    pub as_preferred_username: Option<String>,
    pub as_summary: Option<String>,
    pub as_inbox: String,
    pub as_outbox: String,
    pub as_followers: Option<String>,
    pub as_following: Option<String>,
    pub as_liked: Option<String>,
    pub as_public_key: Value,
    pub as_featured: Option<String>,
    pub as_featured_tags: Option<String>,
    pub as_url: Option<String>,
    pub as_published: Option<DateTime<Utc>>,
    pub as_tag: Value,
    pub as_attachment: Value,
    pub as_endpoints: Value,
    pub as_icon: Value,
    pub as_image: Value,
    pub as_also_known_as: Value,
    pub as_discoverable: bool,
    pub ap_capabilities: Value,
    pub ap_manually_approves_followers: bool,
}

#[derive(Identifiable, Queryable, Serialize, Deserialize, Default, Debug, AsChangeset, Clone)]
#[diesel(table_name = actors)]
pub struct Actor {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ek_uuid: Option<String>,
    pub ek_username: Option<String>,
    pub ek_summary_markdown: Option<String>,
    pub ek_avatar_filename: Option<String>,
    pub ek_banner_filename: Option<String>,
    pub ek_private_key: Option<String>,
    pub ek_password: Option<String>,
    pub ek_client_public_key: Option<String>,
    pub ek_client_private_key: Option<String>,
    pub ek_salt: Option<String>,
    pub ek_olm_pickled_account: Option<String>,
    pub ek_olm_pickled_account_hash: Option<String>,
    pub ek_olm_identity_key: Option<String>,
    pub ek_webfinger: Option<String>,
    pub ek_checked_at: DateTime<Utc>,
    pub ek_hashtags: Value,
    pub as_type: ActorType,
    pub as_context: Option<Value>,
    pub as_id: String,
    pub as_name: Option<String>,
    pub as_preferred_username: Option<String>,
    pub as_summary: Option<String>,
    pub as_inbox: String,
    pub as_outbox: String,
    pub as_followers: Option<String>,
    pub as_following: Option<String>,
    pub as_liked: Option<String>,
    pub as_public_key: Value,
    pub as_featured: Option<String>,
    pub as_featured_tags: Option<String>,
    pub as_url: Option<String>,
    pub as_published: Option<DateTime<Utc>>,
    pub as_tag: Value,
    pub as_attachment: Value,
    pub as_endpoints: Value,
    pub as_icon: Value,
    pub as_image: Value,
    pub as_also_known_as: Value,
    pub as_discoverable: bool,
    pub ap_capabilities: Value,
    pub ap_manually_approves_followers: bool,
}

pub async fn create_or_update_actor(conn: &Db, actor: NewActor) -> Result<Actor> {
    conn.run(move |c| {
        diesel::insert_into(actors::table)
            .values(&actor)
            .on_conflict(actors::as_id)
            .do_update()
            .set(&actor)
            .get_result(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

impl Actor {
    pub fn set_avatar(mut self) -> Self {
        self.ek_avatar_filename = Some(
            self.ek_avatar_filename
                .unwrap_or((*crate::DEFAULT_AVATAR).clone()),
        );

        self.clone()
    }
}

pub async fn update_olm_account_by_username(
    conn: &Db,
    username: String,
    account: String,
    account_hash: String,
) -> Option<Actor> {
    conn.run(move |c| {
        diesel::update(actors::table.filter(actors::ek_username.eq(username)))
            .set((
                actors::ek_olm_pickled_account.eq(account),
                actors::ek_olm_pickled_account_hash.eq(account_hash),
            ))
            .get_result::<Actor>(c)
    })
    .await
    .ok()
}

pub async fn update_avatar_by_username(
    conn: &Db,
    username: String,
    filename: String,
) -> Option<Actor> {
    conn.run(move |c| {
        diesel::update(actors::table.filter(actors::ek_username.eq(username)))
            .set(actors::ek_avatar_filename.eq(filename))
            .get_result::<Actor>(c)
    })
    .await
    .ok()
}

pub async fn update_banner_by_username(
    conn: &Db,
    username: String,
    filename: String,
) -> Option<Actor> {
    conn.run(move |c| {
        diesel::update(actors::table.filter(actors::ek_username.eq(username)))
            .set(actors::ek_banner_filename.eq(filename))
            .get_result::<Actor>(c)
    })
    .await
    .ok()
}

pub async fn update_summary_by_username(
    conn: &Db,
    username: String,
    summary: String,
    summary_markdown: String,
) -> Option<Actor> {
    conn.run(move |c| {
        diesel::update(actors::table.filter(actors::ek_username.eq(username)))
            .set((
                actors::as_summary.eq(summary),
                actors::ek_summary_markdown.eq(summary_markdown),
            ))
            .get_result::<Actor>(c)
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
) -> Option<Actor> {
    conn.run(move |c| {
        diesel::update(actors::table.filter(actors::ek_username.eq(username)))
            .set((
                actors::ek_password.eq(password),
                actors::ek_client_private_key.eq(client_private_key),
                actors::ek_olm_pickled_account.eq(olm_pickled_account),
            ))
            .get_result::<Actor>(c)
    })
    .await
    .ok()
}
