use std::collections::HashSet;

use crate::db::Db;
use deadpool_diesel::postgres::Object as DbConnection;
//use crate::models::leaders::Leader;
use crate::schema::actors;
use crate::{GetHashtags, POOL};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use convert_case::{Case, Casing};
use diesel::prelude::*;
use diesel::sql_query;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use jdt_activity_pub::{
    ApActor, ApActorTerse, ApActorType, ApCollection, ApContext, ApDateTime, ApInstrument,
    ApInstrumentType, Ephemeral, MaybeReference,
};
use jdt_activity_pub::{ApAddress, MaybeMultiple};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::fmt::{self, Debug};

use super::coalesced_activity::CoalescedActivity;
use super::follows::{get_followers_by_actor_id, Follow};
//use super::followers::get_followers_by_actor_id;

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
    Tombstone,
}

impl fmt::Display for ActorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
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

    pub fn is_tombstone(&self) -> bool {
        matches!(self, ActorType::Tombstone)
    }
}

impl From<ActorType> for ApActorType {
    fn from(t: ActorType) -> Self {
        match t {
            ActorType::Application => ApActorType::Application,
            ActorType::Group => ApActorType::Group,
            ActorType::Organization => ApActorType::Organization,
            ActorType::Person => ApActorType::Person,
            ActorType::Service => ApActorType::Service,
            ActorType::Tombstone => ApActorType::Tombstone,
        }
    }
}

impl TryFrom<String> for ActorType {
    type Error = anyhow::Error;

    fn try_from(actor: String) -> Result<Self, Self::Error> {
        match actor.to_case(Case::Snake).as_str() {
            "person" => Ok(ActorType::Person),
            "service" => Ok(ActorType::Service),
            "group" => Ok(ActorType::Group),
            "organization" => Ok(ActorType::Organization),
            "application" => Ok(ActorType::Application),
            _ => Err(anyhow!("unimplemented ActorType")),
        }
    }
}

impl From<ActorType> for String {
    fn from(actor: ActorType) -> Self {
        format!("{actor}").to_case(Case::Snake)
    }
}

impl From<Actor> for Vec<ApInstrument> {
    fn from(actor: Actor) -> Self {
        let mut instruments: Vec<ApInstrument> = vec![];

        if actor.ek_olm_pickled_account.is_some() {
            instruments.push(ApInstrument {
                kind: ApInstrumentType::OlmAccount,
                id: Some(format!("{}#olm-account", actor.as_id)),
                content: actor.ek_olm_pickled_account,
                hash: actor.ek_olm_pickled_account_hash,
                uuid: None,
                name: None,
                url: None,
                mutation_of: None,
                conversation: None,
                activity: None,
            })
        }

        if actor.ek_olm_identity_key.is_some() {
            instruments.push(ApInstrument {
                kind: ApInstrumentType::OlmIdentityKey,
                id: Some(format!("{}#identity-key", actor.as_id)),
                content: actor.ek_olm_identity_key,
                hash: None,
                uuid: None,
                name: None,
                url: None,
                mutation_of: None,
                conversation: None,
                activity: None,
            })
        };

        if actor.ek_mls_credentials.is_some() {
            instruments.push(ApInstrument {
                kind: ApInstrumentType::MlsCredentials,
                id: Some(format!("{}#mls-credentials", actor.as_id)),
                content: actor.ek_mls_credentials,
                hash: None,
                uuid: None,
                name: None,
                url: None,
                mutation_of: None,
                conversation: None,
                activity: None,
            })
        }

        if actor.ek_mls_storage.is_some() {
            instruments.push(ApInstrument {
                kind: ApInstrumentType::MlsStorage,
                id: Some(format!("{}#mls-storage", actor.as_id)),
                content: actor.ek_mls_storage,
                hash: actor.ek_mls_storage_hash,
                uuid: None,
                name: None,
                url: None,
                mutation_of: None,
                conversation: None,
                activity: None,
            })
        }

        instruments
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

    #[cfg(feature = "pg")]
    pub as_type: ActorType,

    #[cfg(feature = "sqlite")]
    pub as_type: String,

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
    pub as_featured: Option<Value>,
    pub as_featured_tags: Option<String>,
    pub as_url: Option<Value>,
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
    pub ek_keys: Option<String>,
}

#[derive(
    Identifiable,
    Queryable,
    QueryableByName,
    Serialize,
    Deserialize,
    Default,
    Debug,
    AsChangeset,
    Clone,
)]
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
    #[serde(skip_serializing)]
    pub ek_private_key: Option<String>,
    #[serde(skip_serializing)]
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

    #[cfg(feature = "pg")]
    pub as_type: ActorType,

    #[cfg(feature = "sqlite")]
    pub as_type: String,

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
    pub as_featured: Option<Value>,
    pub as_featured_tags: Option<String>,
    pub as_url: Option<Value>,
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
    pub ek_keys: Option<String>,
    pub ek_last_decrypted_activity: DateTime<Utc>,
    pub ek_mls_credentials: Option<String>,
    pub ek_mls_storage: Option<String>,
    pub ek_mls_storage_hash: Option<String>,
    pub ek_muted_terms: Value,
}

impl fmt::Display for Actor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_id)
    }
}

impl From<Actor> for ApActorTerse {
    fn from(actor: Actor) -> Self {
        let name = actor.as_name;
        let id = actor.as_id;
        let preferred_username = actor.as_preferred_username.unwrap_or_default();
        let url = actor.as_url.into();
        let icon = actor.as_icon.try_into().ok();
        let tag = actor.as_tag.into();
        let webfinger = actor.ek_webfinger;

        ApActorTerse {
            name,
            id,
            preferred_username,
            url,
            icon,
            tag,
            webfinger,
        }
    }
}

impl FromIterator<Actor> for Vec<ApActor> {
    fn from_iter<I: IntoIterator<Item = Actor>>(iter: I) -> Self {
        iter.into_iter().map(ApActor::from).collect()
    }
}

impl FromIterator<Actor> for Vec<ApActorTerse> {
    fn from_iter<I: IntoIterator<Item = Actor>>(iter: I) -> Self {
        iter.into_iter().map(ApActorTerse::from).collect()
    }
}

pub trait FromExtendedActor {
    fn from_extended_actor(actor: ExtendedActor) -> Self;
}

type ExtendedActor = (Actor, Option<Follow>);

impl FromExtendedActor for ApActor {
    fn from_extended_actor((actor, follow): ExtendedActor) -> Self {
        let mut actor = ApActor::from(actor);

        actor.ephemeral = Some(Ephemeral {
            following: follow.clone().map(|x| x.accepted),
            follow_activity_as_id: follow.and_then(|x| x.follow_activity_ap_id),
            ..Default::default()
        });

        actor
    }
}

impl From<Actor> for ApActor {
    fn from(actor: Actor) -> Self {
        let context = Some(ApContext::full());
        let name = actor.as_name;
        let summary = actor.as_summary;
        let id = Some(actor.as_id.into());
        let kind = actor.as_type.into();
        let preferred_username = actor.as_preferred_username.unwrap_or_default();
        let inbox = actor.as_inbox;
        let outbox = actor.as_outbox;
        let followers = actor.as_followers;
        let following = actor.as_following;
        let featured = match actor.as_featured {
            Some(v) => match v {
                Value::String(s) => MaybeReference::Reference(s),
                _ => {
                    if let Ok(collection) = serde_json::from_value::<ApCollection>(v) {
                        MaybeReference::Actual(collection)
                    } else {
                        MaybeReference::None
                    }
                }
            },
            None => MaybeReference::None,
        };
        let featured_tags = actor.as_featured_tags;
        let manually_approves_followers = Some(actor.ap_manually_approves_followers);
        let published = actor.as_published.map(ApDateTime::from);
        let liked = actor.as_liked;
        let public_key = actor
            .as_public_key
            .try_into()
            .expect("actor must have a public key");
        let url = actor.as_url.into();
        let icon = actor.as_icon.try_into().ok();
        let image = actor.as_image.try_into().ok();
        let discoverable = Some(actor.as_discoverable);
        let capabilities = actor.ap_capabilities.try_into().ok();
        let attachment = actor.as_attachment.into();
        let also_known_as = actor.as_also_known_as.into();
        let tag = actor.as_tag.into();
        let endpoints = actor.as_endpoints.try_into().ok();
        let keys = actor.ek_keys;
        let ephemeral = actor.ek_webfinger.map(|webfinger| Ephemeral {
            webfinger: Some(webfinger),
            ..Default::default()
        });
        let assertion_method = MaybeMultiple::None;
        let generator = None;
        let updated = None;

        ApActor {
            context,
            name,
            summary,
            id,
            kind,
            preferred_username,
            inbox,
            outbox,
            followers,
            following,
            subscribers: None,
            featured,
            featured_tags,
            manually_approves_followers,
            published,
            liked,
            public_key,
            url,
            icon,
            image,
            discoverable,
            capabilities,
            attachment,
            also_known_as,
            tag,
            endpoints,
            keys,
            ephemeral,
            assertion_method,
            generator,
            updated,
        }
    }
}

impl From<&Actor> for ApActor {
    fn from(actor: &Actor) -> ApActor {
        ApActor::from(actor.clone())
    }
}

pub trait FromActorAndLeader {
    fn from_actor_and_leader(actor: ActorAndLeader) -> Self;
}

type ActorAndLeader = (ApActor, Option<Follow>);

impl FromActorAndLeader for ApActor {
    fn from_actor_and_leader((mut actor, follow): ActorAndLeader) -> Self {
        actor.ephemeral = Some(Ephemeral {
            following: follow.clone().map(|x| x.accepted),
            follow_activity_as_id: follow.and_then(|x| x.follow_activity_ap_id),
            ..Default::default()
        });

        actor
    }
}

impl TryFrom<CoalescedActivity> for Actor {
    type Error = anyhow::Error;

    fn try_from(activity: CoalescedActivity) -> Result<Actor> {
        let id = activity.target_actor_id.ok_or(anyhow!("no id"))?;
        let created_at = activity.actor_created_at.ok_or(anyhow!("no created_at"))?;
        let updated_at = activity.actor_updated_at.ok_or(anyhow!("no updated_at"))?;
        let ek_uuid = activity.actor_uuid;
        let ek_username = activity.actor_username;
        let ek_summary_markdown = activity.actor_summary_markdown;
        let ek_avatar_filename = activity.actor_avatar_filename;
        let ek_banner_filename = activity.actor_banner_filename;
        let ek_private_key = activity.actor_private_key;
        let ek_password = activity.actor_password;
        let ek_client_public_key = activity.actor_client_public_key;
        let ek_client_private_key = activity.actor_client_private_key;
        let ek_salt = activity.actor_salt;
        let ek_olm_pickled_account = activity.actor_olm_pickled_account;
        let ek_olm_pickled_account_hash = activity.actor_olm_pickled_account_hash;
        let ek_olm_identity_key = activity.actor_olm_identity_key;
        let ek_webfinger = activity.actor_webfinger;
        let ek_checked_at = activity.actor_checked_at.ok_or(anyhow!("no checked_at"))?;
        let ek_hashtags = activity.actor_hashtags.ok_or(anyhow!("no hashtags"))?;
        let as_type = activity.actor_type.ok_or(anyhow!("no type"))?;
        let as_context = activity.actor_context;
        let as_id = activity.actor_as_id.ok_or(anyhow!("no as_id"))?;
        let as_name = activity.actor_name;
        let as_preferred_username = activity.actor_preferred_username;
        let as_summary = activity.actor_summary;
        let as_inbox = activity.actor_inbox.ok_or(anyhow!("no inbox"))?;
        let as_outbox = activity.actor_outbox.ok_or(anyhow!("no outbox"))?;
        let as_followers = activity.actor_followers;
        let as_following = activity.actor_following;
        let as_liked = activity.actor_liked;
        let as_public_key = activity.actor_public_key.ok_or(anyhow!("no public_key"))?;
        let as_featured = activity.actor_featured;
        let as_featured_tags = activity.actor_featured_tags;
        let as_url = activity.actor_url;
        let as_published = activity.actor_published;
        let as_tag = activity.actor_tag.ok_or(anyhow!("no tag"))?;
        let as_attachment = activity.actor_attachment.ok_or(anyhow!("no attachment"))?;
        let as_endpoints = activity.actor_endpoints.ok_or(anyhow!("no endpoints"))?;
        let as_icon = activity.actor_icon.ok_or(anyhow!("no icon"))?;
        let as_image = activity.actor_image.ok_or(anyhow!("no image"))?;
        let as_also_known_as = activity
            .actor_also_known_as
            .ok_or(anyhow!("no also_known_as"))?;
        let as_discoverable = activity
            .actor_discoverable
            .ok_or(anyhow!("no discoverable"))?;
        let ap_capabilities = activity
            .actor_capabilities
            .ok_or(anyhow!("no capabilities"))?;
        let ap_manually_approves_followers = activity
            .actor_manually_approves_followers
            .ok_or(anyhow!("no manually_approves_followers"))?;
        let ek_keys = activity.actor_keys;
        let ek_last_decrypted_activity = activity
            .actor_last_decrypted_activity
            .ok_or(anyhow!("no last_decrypted_activity"))?;
        let ek_mls_credentials = activity.actor_mls_credentials;
        let ek_mls_storage = activity.actor_mls_storage;
        let ek_mls_storage_hash = activity.actor_mls_storage_hash;
        let ek_muted_terms = activity
            .actor_muted_terms
            .ok_or(anyhow!("no muted terms"))?;

        Ok(Actor {
            id,
            created_at,
            updated_at,
            ek_uuid,
            ek_username,
            ek_summary_markdown,
            ek_avatar_filename,
            ek_banner_filename,
            ek_private_key,
            ek_password,
            ek_client_public_key,
            ek_client_private_key,
            ek_salt,
            ek_olm_pickled_account,
            ek_olm_pickled_account_hash,
            ek_olm_identity_key,
            ek_webfinger,
            ek_checked_at,
            ek_hashtags,
            as_type,
            as_context,
            as_id,
            as_name,
            as_preferred_username,
            as_summary,
            as_inbox,
            as_outbox,
            as_followers,
            as_following,
            as_liked,
            as_public_key,
            as_featured,
            as_featured_tags,
            as_url,
            as_published,
            as_tag,
            as_attachment,
            as_endpoints,
            as_icon,
            as_image,
            as_also_known_as,
            as_discoverable,
            ap_capabilities,
            ek_keys,
            ap_manually_approves_followers,
            ek_last_decrypted_activity,
            ek_mls_credentials,
            ek_mls_storage,
            ek_mls_storage_hash,
            ek_muted_terms,
        })
    }
}

impl Actor {
    pub fn is_stale(&self) -> bool {
        Utc::now() - self.updated_at > Duration::days(7)
    }
}

impl TryFrom<ApActor> for NewActor {
    type Error = anyhow::Error;

    fn try_from(actor: ApActor) -> Result<NewActor, Self::Error> {
        let ek_hashtags = json!(actor.get_hashtags());
        let ek_webfinger = None;
        let ek_checked_at = Utc::now();
        let as_id = actor.id.clone().ok_or(anyhow!("no id"))?.to_string();
        let as_type = actor.kind.to_string().try_into()?;
        let as_context = actor.context.as_ref().map(|x| json!(x));
        let as_name = actor.clone().name;
        let as_preferred_username = Some(actor.preferred_username);
        let as_summary = actor.summary;
        let as_inbox = actor.inbox;
        let as_outbox = actor.outbox;
        let as_followers = actor.followers;
        let as_following = actor.following;
        let as_liked = actor.liked;
        let as_public_key = json!(actor.public_key);
        let as_featured = match actor.featured {
            MaybeReference::Reference(s) => Some(json!(s)),
            MaybeReference::Actual(collection) => Some(json!(collection)),
            MaybeReference::Identifier(id) => Some(json!(id)),
            MaybeReference::None => None,
        };
        let as_featured_tags = actor.featured_tags;
        let as_url = (&actor.url.clone()).into();
        let ap_manually_approves_followers = actor.manually_approves_followers.unwrap_or_default();
        let as_published = actor.published.as_deref().cloned();
        let as_tag = actor.tag.into();
        let as_attachment = actor.attachment.into();
        let as_endpoints = actor.endpoints.map_or(json!({}), |x| json!(x));
        let as_icon = actor.icon.map_or(json!({}), |x| json!(x));
        let as_image = actor.image.map_or(json!({}), |x| json!(x));
        let as_also_known_as = actor.also_known_as.multiple().into();
        let as_discoverable = actor.discoverable.unwrap_or_default();
        let ap_capabilities = actor.capabilities.map_or(json!({}), |x| json!(x));

        Ok(NewActor {
            as_context,
            as_type,
            as_id,
            ek_webfinger,
            as_name,
            as_preferred_username,
            as_summary,
            as_inbox,
            as_outbox,
            as_followers,
            as_following,
            as_liked,
            as_public_key,
            as_featured,
            as_featured_tags,
            as_url,
            ap_manually_approves_followers,
            as_published,
            as_tag,
            as_attachment,
            as_endpoints,
            as_icon,
            as_image,
            as_also_known_as,
            as_discoverable,
            ap_capabilities,
            ek_hashtags,
            ek_checked_at,
            ..Default::default()
        })
    }
}

pub async fn create_or_update_actor(conn: Option<&Db>, actor: NewActor) -> Result<Actor> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(actors::table)
                    .values(&actor)
                    .on_conflict(actors::as_id)
                    .do_update()
                    .set(&actor)
                    .get_result(c)
            })
            .await
            .map_err(anyhow::Error::msg),
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            diesel::insert_into(actors::table)
                .values(&actor)
                .on_conflict(actors::as_id)
                .do_update()
                .set(&actor)
                .get_result(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}

pub async fn set_mls_credentials_by_username(
    conn: &Db,
    username: String,
    credentials: String,
) -> Result<Actor> {
    conn.run(move |c| {
        diesel::update(actors::table)
            .filter(
                actors::ek_username
                    .eq(username)
                    .and(actors::ek_mls_credentials.is_null()),
            )
            .set(actors::ek_mls_credentials.eq(credentials))
            .get_result::<Actor>(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn update_mls_storage_by_username(
    conn: &Db,
    username: String,
    storage: String,
    storage_hash: String,
    mutation_of: Option<String>,
) -> Result<Actor> {
    conn.run(move |c| {
        diesel::update(actors::table)
            .filter(
                actors::ek_username.eq(username).and(
                    actors::ek_mls_storage_hash
                        .eq(mutation_of)
                        .or(actors::ek_mls_storage_hash.is_null()),
                ),
            )
            .set((
                actors::ek_mls_storage.eq(storage),
                actors::ek_mls_storage_hash.eq(storage_hash),
            ))
            .get_result::<Actor>(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn update_olm_account_by_username(
    conn: &Db,
    username: String,
    account: String,
    account_hash: String,
    mutation_of: String,
) -> Result<Actor> {
    conn.run(move |c| {
        diesel::update(actors::table)
            .filter(
                actors::ek_username
                    .eq(username)
                    .and(actors::ek_olm_pickled_account_hash.eq(mutation_of)),
            )
            .set((
                actors::ek_olm_pickled_account.eq(account),
                actors::ek_olm_pickled_account_hash.eq(account_hash),
            ))
            .get_result::<Actor>(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn update_avatar_by_username(
    conn: &Db,
    username: String,
    filename: String,
    as_image: Value,
) -> Option<Actor> {
    conn.run(move |c| {
        diesel::update(actors::table.filter(actors::ek_username.eq(username)))
            .set((
                actors::ek_avatar_filename.eq(filename),
                actors::as_icon.eq(as_image),
            ))
            .get_result::<Actor>(c)
    })
    .await
    .ok()
}

pub async fn update_banner_by_username(
    conn: &Db,
    username: String,
    filename: String,
    as_image: Value,
) -> Option<Actor> {
    conn.run(move |c| {
        diesel::update(actors::table.filter(actors::ek_username.eq(username)))
            .set((
                actors::ek_banner_filename.eq(filename),
                actors::as_image.eq(as_image),
            ))
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
    olm_pickled_account_hash: String,
) -> Option<Actor> {
    conn.run(move |c| {
        diesel::update(actors::table.filter(actors::ek_username.eq(username)))
            .set((
                actors::ek_password.eq(password),
                actors::ek_client_private_key.eq(client_private_key),
                actors::ek_olm_pickled_account.eq(olm_pickled_account),
                actors::ek_olm_pickled_account_hash.eq(olm_pickled_account_hash),
            ))
            .get_result::<Actor>(c)
    })
    .await
    .ok()
}

pub async fn get_muted_terms_by_username(
    conn_opt: Option<&Db>,
    username: String,
) -> Result<Vec<String>, anyhow::Error> {
    let operation = move |c: &mut PgConnection| {
        actors::table
            .filter(actors::ek_username.eq(username))
            .select(actors::ek_muted_terms)
            .first::<Value>(c)
            .map(|terms| {
                terms
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .map_err(anyhow::Error::from)
    };

    match conn_opt {
        Some(conn) => conn.run(operation).await,
        None => {
            tokio::task::spawn_blocking(move || {
                let mut pool_conn = POOL.get().map_err(anyhow::Error::msg)?;
                operation(&mut pool_conn)
            })
            .await?
        }
    }
}

pub async fn update_muted_terms_by_username(
    conn_opt: Option<&Db>,
    username: String,
    terms: Vec<String>,
) -> Result<Actor, anyhow::Error> {
    let terms_json = json!(terms);

    let operation = move |c: &mut PgConnection| {
        diesel::update(actors::table)
            .filter(actors::ek_username.eq(username))
            .set(actors::ek_muted_terms.eq(terms_json))
            .get_result::<Actor>(c)
            .map_err(anyhow::Error::from)
    };

    match conn_opt {
        Some(conn) => conn.run(operation).await,
        None => {
            tokio::task::spawn_blocking(move || {
                let mut pool_conn = POOL.get().map_err(anyhow::Error::msg)?;
                operation(&mut pool_conn)
            })
            .await?
        }
    }
}

pub async fn get_actor_by_key_id(conn: &Db, key_id: String) -> Result<Actor> {
    use diesel::sql_types::Text;

    conn.run(move |c: &mut PgConnection| {
        sql_query("SELECT * FROM actors WHERE as_public_key->>'id' = $1 LIMIT 1")
            .bind::<Text, _>(key_id.clone())
            .get_result::<Actor>(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn tombstone_actor_by_as_id(conn_opt: Option<&Db>, as_id: String) -> Result<Actor> {
    let operation = move |c: &mut PgConnection| {
        diesel::update(actors::table.filter(actors::as_id.eq(as_id)))
            .set((
                actors::as_type.eq(ActorType::Tombstone),
                // Clear user-supplied content fields
                actors::as_name.eq(None::<String>),
                actors::as_summary.eq(None::<String>),
                actors::ek_summary_markdown.eq(None::<String>),
                actors::as_attachment.eq(json!([])),
                actors::as_tag.eq(json!([])),
                actors::ek_hashtags.eq(json!([])),
                // Clear avatar and banner
                actors::ek_avatar_filename.eq(None::<String>),
                actors::ek_banner_filename.eq(None::<String>),
                actors::as_icon.eq(json!({})),
                actors::as_image.eq(json!({})),
                // Clear discovery and social fields
                actors::as_discoverable.eq(false),
                actors::as_featured.eq(None::<Value>),
                actors::as_featured_tags.eq(None::<String>),
                actors::as_also_known_as.eq(json!([])),
                // Clear capabilities
                actors::ap_capabilities.eq(json!({})),
                // Clear encryption-related fields
                actors::ek_client_public_key.eq(None::<String>),
                actors::ek_client_private_key.eq(None::<String>),
                actors::ek_olm_pickled_account.eq(None::<String>),
                actors::ek_olm_pickled_account_hash.eq(None::<String>),
                actors::ek_olm_identity_key.eq(None::<String>),
                actors::ek_mls_credentials.eq(None::<String>),
                actors::ek_mls_storage.eq(None::<String>),
                actors::ek_mls_storage_hash.eq(None::<String>),
                actors::ek_keys.eq(None::<String>),
                // Clear muted terms
                actors::ek_muted_terms.eq(json!([])),
            ))
            .get_result(c)
    };

    crate::db::run_db_op(conn_opt, &crate::POOL, operation).await
}

pub async fn delete_actor_by_as_id(conn: &Db, as_id: String) -> bool {
    // This function checks if ek_username is null to avoid deleting local user records
    conn.run(move |c| {
        diesel::delete(
            actors::table.filter(actors::as_id.eq(as_id).and(actors::ek_username.is_null())),
        )
        .execute(c)
    })
    .await
    .is_ok()
}

pub async fn delete_actors_by_domain_pattern(
    conn: Option<&Db>,
    domain_pattern: String,
) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        // First delete activities that reference actors from this domain
        sql_query("DELETE FROM activities WHERE target_actor_id IN (SELECT id FROM actors WHERE as_id COLLATE \"C\" LIKE $1)")
            .bind::<Text, _>(format!("https://{}/%", domain_pattern.clone()))
            .execute(c)?;

        // Then delete the actors themselves
        sql_query("DELETE FROM actors WHERE as_id COLLATE \"C\" LIKE $1")
            .bind::<Text, _>(format!("https://{domain_pattern}/%"))
            .execute(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}

pub async fn get_actor(conn: &Db, id: i32) -> Option<Actor> {
    conn.run(move |c| actors::table.find(id).first::<Actor>(c))
        .await
        .ok()
}

pub async fn get_actor_by_username(conn_opt: Option<&Db>, username: String) -> Result<Actor> {
    let operation = move |c: &mut PgConnection| {
        actors::table
            .filter(actors::ek_username.eq(username))
            .first::<Actor>(c)
            .map_err(anyhow::Error::from)
    };
    match conn_opt {
        Some(conn) => conn.run(operation).await,
        None => {
            tokio::task::spawn_blocking(move || {
                let mut pool_conn = POOL.get().map_err(anyhow::Error::msg)?;
                operation(&mut pool_conn)
            })
            .await?
        }
    }
}

pub async fn get_actor_by_webfinger(conn_opt: Option<&Db>, webfinger: String) -> Result<Actor> {
    let operation = move |c: &mut PgConnection| {
        actors::table
            .filter(actors::ek_webfinger.eq(webfinger))
            .first::<Actor>(c)
            .map_err(anyhow::Error::from)
    };

    match conn_opt {
        Some(conn) => conn.run(operation).await,
        None => {
            tokio::task::spawn_blocking(move || {
                let mut pool_conn = POOL.get().map_err(anyhow::Error::msg)?;
                operation(&mut pool_conn)
            })
            .await?
        }
    }
}

pub async fn get_actor_by_uuid(conn_opt: Option<&Db>, uuid: String) -> Result<Actor> {
    let operation = move |c: &mut PgConnection| {
        actors::table
            .filter(actors::ek_uuid.eq(uuid))
            .first::<Actor>(c)
            .map_err(anyhow::Error::from)
    };
    match conn_opt {
        Some(conn) => conn.run(operation).await,
        None => {
            tokio::task::spawn_blocking(move || {
                let mut pool_conn = POOL.get().map_err(anyhow::Error::msg)?;
                operation(&mut pool_conn)
            })
            .await?
        }
    }
}

pub async fn get_actor_by_as_id(conn_opt: Option<&Db>, as_id: String) -> Result<Actor> {
    let operation = move |c: &mut PgConnection| {
        actors::table
            .filter(actors::as_id.eq(as_id))
            .first::<Actor>(c)
            .map_err(anyhow::Error::from)
    };
    match conn_opt {
        Some(conn) => conn.run(operation).await,
        None => {
            tokio::task::spawn_blocking(move || {
                let mut pool_conn = POOL.get().map_err(anyhow::Error::msg)?;
                operation(&mut pool_conn)
            })
            .await?
        }
    }
}

pub async fn get_follower_inboxes(conn: &Db, actor: Actor) -> Vec<ApAddress> {
    let mut inboxes: HashSet<ApAddress> = HashSet::new();

    for (_follower, actor) in get_followers_by_actor_id(Some(conn), actor.id, None).await {
        inboxes.insert(ApAddress::Address(actor.as_inbox));
    }

    Vec::from_iter(inboxes)
}

pub async fn guaranteed_actor(conn: &Db, profile: Option<Actor>) -> Actor {
    match profile {
        Some(profile) => profile,
        // Pass Some(conn) here as guaranteed_actor is likely called from server context
        None => get_actor_by_username(Some(conn), (*crate::SYSTEM_USER).clone())
            .await // get_actor_by_username now returns Result<Actor>
            .expect("Unable to retrieve system user"),
    }
}

pub async fn get_actor_by_key_id_axum(conn: &DbConnection, key_id: String) -> Result<Actor> {
    conn.interact(move |c| {
        use diesel::sql_types::Text;
        diesel::sql_query("SELECT * FROM actors WHERE as_public_key->>'id' = $1 LIMIT 1")
            .bind::<Text, _>(key_id)
            .get_result::<Actor>(c)
    })
    .await
    .map_err(|e| anyhow::anyhow!("Interact error: {:?}", e))?
    .map_err(anyhow::Error::from)
}

pub async fn get_actor_by_username_axum(conn: &DbConnection, username: String) -> Result<Actor> {
    conn.interact(move |c| {
        use crate::schema::actors::dsl::*;
        actors.filter(ek_username.eq(username)).first::<Actor>(c)
    })
    .await
    .map_err(|e| anyhow::anyhow!("Interact error: {:?}", e))?
    .map_err(anyhow::Error::from)
}
