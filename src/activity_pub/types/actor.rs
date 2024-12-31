use core::fmt;
use std::fmt::Debug;

use super::activity::ApActivity;
use super::Ephemeral;
use crate::activity_pub::{ApAttachment, ApContext, ApEndpoint, ApImage, ApTag};
use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::models::actors::ActorType;
use crate::models::actors::{get_actor_by_as_id, Actor};
use crate::models::followers::get_follower_count_by_actor_id;
use crate::models::leaders::{get_leader_by_actor_id_and_ap_id, get_leader_count_by_actor_id};
use crate::routes::ActivityJson;
use crate::webfinger::retrieve_webfinger;
use crate::{MaybeMultiple, DOMAIN_RE};
use anyhow::{self, Result};
use lazy_static::lazy_static;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::serde_as;

lazy_static! {
    pub static ref PUBLIC_COLLECTION: Vec<String> = {
        vec![
            "https://www.w3.org/ns/activitystreams#Public".to_string(),
            "as:Public".to_string(),
            "Public".to_string(),
        ]
    };
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Default, Hash, Ord, PartialOrd)]
#[serde(untagged)]
pub enum ApAddress {
    Address(String),
    #[default]
    None,
}

impl ApAddress {
    pub fn is_public(&self) -> bool {
        if let ApAddress::Address(x) = self {
            x.to_lowercase() == *"https://www.w3.org/ns/activitystreams#public"
        } else {
            false
        }
    }

    pub fn get_public() -> Self {
        ApAddress::Address("https://www.w3.org/ns/activitystreams#Public".to_string())
    }
}

impl fmt::Display for ApAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let ApAddress::Address(x) = self {
            write!(f, "{}", x.clone())
        } else {
            write!(f, "https://localhost")
        }
    }
}

impl From<String> for ApAddress {
    fn from(address: String) -> Self {
        ApAddress::Address(address)
    }
}

impl TryFrom<Value> for ApAddress {
    type Error = anyhow::Error;

    fn try_from(value: Value) -> Result<Self> {
        serde_json::from_value(value).map_err(anyhow::Error::msg)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct ApPublicKey {
    pub id: String,
    pub owner: String,
    pub public_key_pem: String,
}

impl TryFrom<Value> for ApPublicKey {
    type Error = anyhow::Error;

    fn try_from(value: Value) -> Result<Self> {
        serde_json::from_value(value).map_err(anyhow::Error::msg)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct ApCapabilities {
    pub accepts_chat_messages: Option<bool>,
    pub enigmatick_encryption: Option<bool>,
}

impl TryFrom<Value> for ApCapabilities {
    type Error = anyhow::Error;

    fn try_from(value: Value) -> Result<Self> {
        serde_json::from_value(value).map_err(anyhow::Error::msg)
    }
}

#[derive(Serialize, PartialEq, Eq, Deserialize, Clone, Debug, Default, Hash, Ord, PartialOrd)]
pub enum ApActorType {
    #[serde(alias = "application")]
    Application,
    #[serde(alias = "group")]
    Group,
    #[serde(alias = "organization")]
    Organization,
    #[serde(alias = "person")]
    Person,
    #[serde(alias = "service")]
    Service,
    #[serde(alias = "tombstone")]
    Tombstone,
    #[default]
    #[serde(alias = "unknown")]
    Unknown,
}

impl fmt::Display for ApActorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
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

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct ApActor {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,

    #[serde(rename = "type")]
    pub kind: ApActorType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<ApAddress>,

    pub preferred_username: String,
    pub inbox: String,
    pub outbox: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub followers: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribers: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub liked: Option<String>,

    pub public_key: ApPublicKey,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured_tags: Option<String>,

    // BlueSky seems to use an array here (or the bridges do)
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub url: MaybeMultiple<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub manually_approves_followers: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,

    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub tag: MaybeMultiple<ApTag>,

    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub attachment: MaybeMultiple<ApAttachment>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<ApEndpoint>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<ApImage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ApImage>,

    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub also_known_as: MaybeMultiple<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub discoverable: Option<bool>,

    // perhaps SoapBox/Pleroma-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ApCapabilities>,

    // Enigmatick-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keys: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<Ephemeral>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct ApActorTerse {
    pub id: String,
    pub url: MaybeMultiple<String>,
    pub name: Option<String>,
    pub preferred_username: String,
    pub tag: MaybeMultiple<ApTag>,
    pub icon: Option<ApImage>,
}

impl Default for ApActor {
    fn default() -> ApActor {
        ApActor {
            context: Some(ApContext::Plain(
                "https://www.w3.org/ns/activitystreams".to_string(),
            )),
            kind: ApActorType::default(),
            name: None,
            summary: None,
            id: None,
            preferred_username: String::new(),
            inbox: String::new(),
            outbox: String::new(),
            followers: None,
            following: None,
            subscribers: None,
            liked: None,
            public_key: ApPublicKey::default(),
            featured: None,
            featured_tags: None,
            url: MaybeMultiple::None,
            manually_approves_followers: None,
            published: None,
            tag: MaybeMultiple::None,
            attachment: MaybeMultiple::None,
            endpoints: None,
            icon: None,
            image: None,
            also_known_as: MaybeMultiple::None,
            discoverable: None,
            capabilities: None,
            keys: None,
            ephemeral: None,
        }
    }
}

impl ApActor {
    pub async fn load_ephemeral(&mut self, conn: &Db, requester: Option<Actor>) -> Self {
        if let Some(ap_id) = self.id.clone() {
            if let Ok(profile) = get_actor_by_as_id(conn, ap_id.to_string()).await {
                self.ephemeral = Some(Ephemeral {
                    followers: get_follower_count_by_actor_id(conn, profile.id).await.ok(),
                    leaders: get_leader_count_by_actor_id(conn, profile.id).await.ok(),
                    summary_markdown: profile.ek_summary_markdown,
                    following: {
                        if let Some(requester) = requester {
                            if let Some(id) = self.id.clone() {
                                get_leader_by_actor_id_and_ap_id(conn, requester.id, id.to_string())
                                    .await
                                    .and_then(|x| x.accepted)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    },
                    ..Default::default()
                });
            }
        }

        self.clone()
    }

    pub async fn get_webfinger(&self) -> Option<String> {
        let id = self.id.clone()?.to_string();
        let domain = DOMAIN_RE.captures(&id)?.get(1)?.as_str().to_string();
        let username = self.preferred_username.clone();

        let webfinger = retrieve_webfinger(domain, username).await.ok()?;

        webfinger.get_address()
    }

    pub fn get_hashtags(&self) -> Vec<String> {
        if let MaybeMultiple::Multiple(tags) = self.tag.clone() {
            tags.iter()
                .filter_map(|tag| {
                    if let ApTag::HashTag(hashtag) = tag {
                        Some(hashtag.name.clone().to_lowercase())
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            vec![]
        }
    }
}

impl FromIterator<ApActor> for Vec<ApActorTerse> {
    fn from_iter<I: IntoIterator<Item = ApActor>>(iter: I) -> Self {
        iter.into_iter().map(ApActorTerse::from).collect()
    }
}

impl From<ApActor> for ApActorTerse {
    fn from(actor: ApActor) -> Self {
        ApActorTerse {
            id: actor.id.unwrap_or_default().to_string(),
            url: actor.url,
            name: actor.name,
            preferred_username: actor.preferred_username,
            tag: actor.tag,
            icon: actor.icon,
        }
    }
}
