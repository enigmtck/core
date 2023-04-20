use core::fmt;
use std::fmt::Debug;

use crate::activity_pub::{ApAttachment, ApContext, ApEndpoint, ApImage, ApImageType, ApTag};
use crate::models::leaders::Leader;
use crate::models::profiles::Profile;
use crate::models::remote_actors::RemoteActor;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Default)]
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApPublicKey {
    pub id: String,
    pub owner: String,
    pub public_key_pem: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApCapabilities {
    pub accepts_chat_messages: Option<bool>,
    pub enigmatick_encryption: Option<bool>,
}

#[derive(Serialize, PartialEq, Eq, Deserialize, Clone, Debug, Default)]
pub enum ApActorType {
    Application,
    Group,
    Organization,
    Person,
    Service,
    #[default]
    Unknown,
}

impl fmt::Display for ApActorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
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
    pub also_known_as: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discoverable: Option<bool>,

    // perhaps SoapBox/Pleroma-specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ApCapabilities>,

    // These are ephemeral attributes to facilitate client operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_following: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_leader_ap_id: Option<String>,
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
        }
    }
}

impl From<Profile> for ApActor {
    fn from(profile: Profile) -> Self {
        let server_url = &*crate::SERVER_URL;

        ApActor {
            name: Some(profile.display_name),
            summary: Some(profile.summary.unwrap_or_default()),
            id: Some(ApAddress::Address(format!(
                "{}/user/{}",
                server_url, profile.username
            ))),
            kind: ApActorType::Person,
            preferred_username: profile.username.clone(),
            inbox: format!("{}/user/{}/inbox/", server_url, profile.username),
            outbox: format!("{}/user/{}/outbox/", server_url, profile.username),
            followers: Some(format!(
                "{}/user/{}/followers/",
                server_url, profile.username
            )),
            following: Some(format!(
                "{}/user/{}/following/",
                server_url, profile.username
            )),
            liked: Some(format!("{}/user/{}/liked/", server_url, profile.username)),
            public_key: ApPublicKey {
                id: format!("{}/user/{}#main-key", server_url, profile.username),
                owner: format!("{}/user/{}", server_url, profile.username),
                public_key_pem: profile.public_key,
            },
            url: Some(format!("{}/@{}", server_url, profile.username)),
            icon: Some(ApImage {
                kind: ApImageType::Image,
                media_type: Some("image/png".to_string()),
                url: format!("{server_url}/media/avatars/{}", profile.avatar_filename),
            }),
            image: profile.banner_filename.map(|banner| ApImage {
                kind: ApImageType::Image,
                media_type: Some("image/png".to_string()),
                url: format!("{server_url}/media/banners/{banner}"),
            }),
            discoverable: Some(true),
            capabilities: Some(ApCapabilities {
                accepts_chat_messages: Some(false),
                enigmatick_encryption: Some(true),
            }),
            attachment: Some(vec![]),
            also_known_as: Some(vec![]),
            tag: Some(vec![]),
            endpoints: Some(ApEndpoint {
                shared_inbox: format!("{server_url}/inbox"),
            }),
            ..Default::default()
        }
    }
}

impl From<RemoteActor> for ApActor {
    fn from(actor: RemoteActor) -> Self {
        ApActor {
            context: Some(ApContext::Plain(
                "https://www.w3.org/ns/activitystreams".to_string(),
            )),
            kind: ApActorType::Person,
            name: Some(actor.name),
            summary: actor.summary,
            id: Some(ApAddress::Address(actor.ap_id)),
            preferred_username: actor.preferred_username.unwrap_or_default(),
            inbox: actor.inbox,
            outbox: actor.outbox,
            followers: actor.followers,
            following: actor.following,
            liked: actor.liked,
            public_key: serde_json::from_value(actor.public_key.into()).unwrap(),
            featured: actor.featured,
            featured_tags: actor.featured_tags,
            url: actor.url,
            manually_approves_followers: actor.manually_approves_followers,
            published: actor.published,
            tag: serde_json::from_value(actor.tag.into()).unwrap(),
            attachment: serde_json::from_value(actor.attachment.into()).unwrap(),
            endpoints: serde_json::from_value(actor.endpoints.into()).unwrap(),
            icon: serde_json::from_value(actor.icon.into()).unwrap(),
            image: serde_json::from_value(actor.image.into()).unwrap(),
            also_known_as: serde_json::from_value(actor.also_known_as.into()).unwrap(),
            discoverable: actor.discoverable,
            capabilities: serde_json::from_value(actor.capabilities.into()).unwrap(),
            ephemeral_following: None,
            ephemeral_leader_ap_id: None,
        }
    }
}

type ActorAndLeader = (ApActor, Option<Leader>);

impl From<ActorAndLeader> for ApActor {
    fn from(actor_and_leader: ActorAndLeader) -> Self {
        let mut actor: ApActor = actor_and_leader.0;

        actor.ephemeral_following = {
            if let Some(leader) = actor_and_leader.1.clone() {
                leader.accepted
            } else {
                None
            }
        };

        actor.ephemeral_leader_ap_id = {
            if let Some(leader) = actor_and_leader.1 {
                format!("{}/leader/{}", *crate::SERVER_URL, leader.uuid).into()
            } else {
                None
            }
        };

        actor
    }
}
