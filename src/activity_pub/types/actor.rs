use core::fmt;
use std::fmt::Debug;

use super::activity::ApActivity;
use super::Ephemeral;
use crate::activity_pub::{
    ActivityPub, ApAttachment, ApContext, ApEndpoint, ApImage, ApTag, Outbox,
};
use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::models::actors::ActorType;
use crate::models::actors::{get_actor_by_as_id, Actor};
use crate::models::cache::{cache_content, Cache};
use crate::models::followers::get_follower_count_by_actor_id;
use crate::models::leaders::{get_leader_count_by_actor_id, Leader};
use crate::models::{from_serde, from_serde_option};
use crate::routes::ActivityJson;
use crate::webfinger::retrieve_webfinger;
use crate::{MaybeMultiple, DOMAIN_RE};
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

impl TryFrom<serde_json::Value> for ApAddress {
    type Error = String;

    fn try_from(address: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(address)
            .map_err(|e| format!("FAILED TO CONVERT TO ApAddress: {e:#?}"))?
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
    pub tag: Vec<ApTag>,
    pub icon: Option<ApImage>,
}

impl From<ApActor> for ApActorTerse {
    fn from(actor: ApActor) -> Self {
        let name = actor.name;
        let id = actor.id.unwrap_or_default().to_string();
        let preferred_username = actor.preferred_username;
        let url = actor.url;
        let icon = actor.icon;
        let tag = actor.tag.unwrap_or_default();

        ApActorTerse {
            name,
            id,
            preferred_username,
            url,
            icon,
            tag,
        }
    }
}

impl From<Actor> for ApActorTerse {
    fn from(actor: Actor) -> Self {
        let name = actor.as_name;
        let id = actor.as_id;
        let preferred_username = actor.as_preferred_username.unwrap_or_default();
        let url = from_serde_option(actor.as_url).unwrap_or_default();
        let icon = from_serde(actor.as_icon);
        let tag = from_serde(actor.as_tag).unwrap_or(vec![]);

        ApActorTerse {
            name,
            id,
            preferred_username,
            url,
            icon,
            tag,
        }
    }
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
        _raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
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
            url: MaybeMultiple::None,
            manually_approves_followers: None,
            published: None,
            tag: None,
            attachment: None,
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
    pub async fn load_ephemeral(&mut self, conn: &Db) -> Self {
        if let Some(ap_id) = self.id.clone() {
            if let Ok(profile) = get_actor_by_as_id(conn, ap_id.to_string()).await {
                self.ephemeral = Some(Ephemeral {
                    followers: get_follower_count_by_actor_id(conn, profile.id).await.ok(),
                    leaders: get_leader_count_by_actor_id(conn, profile.id).await.ok(),
                    summary_markdown: profile.ek_summary_markdown,
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

        actor.ephemeral = Some(Ephemeral {
            following: leader.clone().and_then(|x| x.accepted),
            leader_as_id: leader
                .clone()
                .and_then(|x| format!("{}/leader/{}", *crate::SERVER_URL, x.uuid).into()),
            follow_activity_as_id: leader.and_then(|x| x.follow_ap_id),
            ..Default::default()
        });

        actor
    }
}

impl FromIterator<ApActor> for Vec<ApActorTerse> {
    fn from_iter<I: IntoIterator<Item = ApActor>>(iter: I) -> Self {
        iter.into_iter().map(ApActorTerse::from).collect()
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

impl From<Actor> for ApActor {
    fn from(actor: Actor) -> Self {
        let context = Some(from_serde_option(actor.as_context).unwrap_or(ApContext::default()));
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
        let published = actor.as_published.map(ActivityPub::time);
        let liked = actor.as_liked;
        let public_key = from_serde(actor.as_public_key).unwrap();
        let url = actor.as_url.into();
        let icon = from_serde(actor.as_icon);
        let image = from_serde(actor.as_image);
        let discoverable = Some(actor.as_discoverable);
        let capabilities = from_serde(actor.ap_capabilities);
        let attachment = from_serde(actor.as_attachment);
        let also_known_as = actor.as_also_known_as.into();
        let tag = from_serde(actor.as_tag);
        let endpoints = from_serde(actor.as_endpoints);
        let keys = actor.ek_keys;

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
            ephemeral: None,
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
        actor.ephemeral = Some(Ephemeral {
            following: leader.clone().and_then(|x| x.accepted),
            leader_as_id: leader
                .clone()
                .and_then(|x| format!("{}/leader/{}", *crate::SERVER_URL, x.uuid).into()),
            follow_activity_as_id: leader.and_then(|x| x.follow_ap_id),
            ..Default::default()
        });

        actor
    }
}
