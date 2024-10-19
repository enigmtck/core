use core::fmt;
use std::fmt::Debug;

use crate::activity_pub::{ApAttachment, ApContext, ApEndpoint, ApImage, ApTag, Outbox};
use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::models::actors::ActorType;
use crate::models::actors::{get_actor_by_as_id, Actor};
use crate::models::cache::{cache_content, Cache};
use crate::models::followers::get_followers_by_actor_id;
use crate::models::leaders::{get_leaders_by_actor_id, Leader};
use crate::models::{from_serde, from_serde_option};
use crate::{MaybeMultiple, DOMAIN_RE};
use lazy_static::lazy_static;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
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

impl TryFrom<serde_json::Value> for ApAddress {
    type Error = String;

    fn try_from(address: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(address).map_err(|_| "failed to convert to ApAddress")?
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct ApPublicKey {
    pub id: String,
    pub owner: String,
    pub public_key_pem: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct ApCapabilities {
    pub accepts_chat_messages: Option<bool>,
    pub enigmatick_encryption: Option<bool>,
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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub manually_approves_followers: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<Vec<ApTag>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment: Option<Vec<ApAttachment>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<ApEndpoint>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<ApImage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ApImage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub also_known_as: Option<MaybeMultiple<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub discoverable: Option<bool>,

    // perhaps SoapBox/Pleroma-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ApCapabilities>,

    // These facilitate consolidation of joined tables in to this object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_followers: Option<Vec<ApActor>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_leaders: Option<Vec<ApActor>>,

    // These are ephemeral attributes to facilitate client operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_following: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_leader_ap_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_follow_activity_ap_id: Option<String>,

    // These are used for user operations on their own profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_summary_markdown: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ApActorTerse {
    pub id: String,
    pub url: String,
    pub name: String,
    pub tag: Vec<ApTag>,
    pub icon: Option<ApImage>,
}

impl Cache for ApActor {
    async fn cache(&self, conn: &Db) -> &Self {
        if let Some(tags) = self.tag.clone() {
            for tag in tags {
                cache_content(conn, tag.try_into()).await;
            }
        };

        for image in vec![self.image.clone(), self.icon.clone()]
            .into_iter()
            .flatten()
        {
            cache_content(conn, Ok(image.clone().into())).await;
        }

        self
    }
}

impl Outbox for ApActor {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Actor,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
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
            url: None,
            manually_approves_followers: None,
            published: None,
            tag: None,
            attachment: None,
            endpoints: None,
            icon: None,
            image: None,
            also_known_as: None,
            discoverable: None,
            capabilities: None,
            ephemeral_following: None,
            ephemeral_leader_ap_id: None,
            ephemeral_summary_markdown: None,
            ephemeral_followers: None,
            ephemeral_leaders: None,
            ephemeral_follow_activity_ap_id: None,
        }
    }
}

impl ApActor {
    pub async fn load_ephemeral(&mut self, conn: &Db) -> Self {
        if let Some(ap_id) = self.id.clone() {
            if let Some(profile) = get_actor_by_as_id(conn, ap_id.to_string()).await {
                self.ephemeral_followers = Some(
                    get_followers_by_actor_id(conn, profile.id)
                        .await
                        .iter()
                        .filter_map(|(_, remote_actor)| {
                            remote_actor
                                .as_ref()
                                .map(|remote_actor| remote_actor.clone().into())
                        })
                        .collect(),
                );

                self.ephemeral_leaders = Some(
                    get_leaders_by_actor_id(conn, profile.id)
                        .await
                        .iter()
                        .filter_map(|(_, remote_actor)| {
                            remote_actor
                                .as_ref()
                                .map(|remote_actor| remote_actor.clone().into())
                        })
                        .collect(),
                );

                self.ephemeral_summary_markdown = profile.ek_summary_markdown;
            }
        }

        self.clone()
    }

    pub fn get_webfinger(&self) -> Option<String> {
        let id = self.id.clone()?.to_string();
        let server_name = DOMAIN_RE.captures(&id)?.get(1)?.as_str();
        Some(format!("@{}@{}", self.preferred_username, server_name))
    }

    pub fn get_hashtags(&self) -> Vec<String> {
        if let Some(tags) = self.tag.clone() {
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

type ExtendedActor = (Actor, Option<Leader>);

impl From<ExtendedActor> for ApActor {
    fn from((actor, leader): ExtendedActor) -> Self {
        let mut actor = ApActor::from(actor);

        actor.ephemeral_following = leader.clone().and_then(|x| x.accepted);

        actor.ephemeral_leader_ap_id = leader
            .clone()
            .and_then(|x| format!("{}/leader/{}", *crate::SERVER_URL, x.uuid).into());

        actor.ephemeral_follow_activity_ap_id = leader.and_then(|x| x.follow_ap_id);

        actor
    }
}

impl From<Actor> for ApActor {
    fn from(actor: Actor) -> Self {
        let context = from_serde_option(actor.as_context);
        let name = actor.as_name;
        let summary = actor.as_summary;
        let id = Some(actor.as_id.into());
        let kind = actor.as_type.into();
        let preferred_username = actor.as_preferred_username.unwrap_or_default();
        let inbox = actor.as_inbox;
        let outbox = actor.as_outbox;
        let followers = actor.as_followers;
        let following = actor.as_following;
        let featured = actor.as_featured;
        let featured_tags = actor.as_featured_tags;
        let manually_approves_followers = Some(actor.ap_manually_approves_followers);
        let published = actor.as_published.map(|x| x.to_rfc3339());
        let liked = actor.as_liked;
        let public_key = from_serde(actor.as_public_key).unwrap();
        let url = actor.as_url;
        let icon = from_serde(actor.as_icon);
        let image = from_serde(actor.as_image);
        let discoverable = Some(actor.as_discoverable);
        let capabilities = from_serde(actor.ap_capabilities);
        let attachment = from_serde(actor.as_attachment);
        let also_known_as = from_serde(actor.as_also_known_as);
        let tag = from_serde(actor.as_tag);
        let endpoints = from_serde(actor.as_endpoints);

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
            ephemeral_summary_markdown: None,
            ephemeral_following: None,
            ephemeral_leader_ap_id: None,
            ephemeral_followers: None,
            ephemeral_leaders: None,
            ephemeral_follow_activity_ap_id: None,
        }
    }
}

impl From<&Actor> for ApActor {
    fn from(actor: &Actor) -> ApActor {
        ApActor::from(actor.clone())
    }
}

type ActorAndLeader = (ApActor, Option<Leader>);

impl From<ActorAndLeader> for ApActor {
    fn from((mut actor, leader): ActorAndLeader) -> Self {
        actor.ephemeral_following = leader.clone().and_then(|x| x.accepted);

        actor.ephemeral_leader_ap_id = leader
            .clone()
            .and_then(|x| format!("{}/leader/{}", *crate::SERVER_URL, x.uuid).into());

        actor.ephemeral_follow_activity_ap_id = leader.and_then(|x| x.follow_ap_id);

        actor
    }
}
