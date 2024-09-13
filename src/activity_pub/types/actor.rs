use core::fmt;
use std::fmt::Debug;

use crate::activity_pub::{
    ApAttachment, ApContext, ApEndpoint, ApImage, ApImageType, ApTag, Outbox,
};
use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::models::cache::{cache_content, Cache};
use crate::models::followers::get_followers_by_profile_id;
use crate::models::leaders::{get_leaders_by_profile_id, Leader};
use crate::models::profiles::{get_profile_by_ap_id, Profile};
use crate::models::remote_actors::RemoteActor;
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
        _profile: Profile,
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
            if let Some(profile) = get_profile_by_ap_id(Some(conn), ap_id.to_string()).await {
                self.ephemeral_followers = Some(
                    get_followers_by_profile_id(Some(conn), profile.id)
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
                    get_leaders_by_profile_id(conn, profile.id)
                        .await
                        .iter()
                        .filter_map(|(_, remote_actor)| {
                            remote_actor
                                .as_ref()
                                .map(|remote_actor| remote_actor.clone().into())
                        })
                        .collect(),
                );

                self.ephemeral_summary_markdown = profile.summary_markdown;
            }
        }

        self.clone()
    }

    pub fn get_webfinger(&self) -> Option<String> {
        let id = self.id.clone()?.to_string();
        let server_name = DOMAIN_RE.captures(&id)?.get(1)?.as_str();
        Some(format!("@{}@{}", self.preferred_username, server_name))
    }
}

type ExtendedProfile = (Profile, Option<Leader>);

impl From<ExtendedProfile> for ApActor {
    fn from((profile, leader): ExtendedProfile) -> Self {
        let mut actor = ApActor::from(profile);
        if leader.is_some() {
            actor.ephemeral_following = Some(true);
        }

        actor
    }
}

impl From<Profile> for ApActor {
    fn from(profile: Profile) -> Self {
        let server_url = &*crate::SERVER_URL;

        ApActor {
            context: Some(ApContext::default()),
            name: Some(profile.display_name),
            summary: profile.summary,
            id: Some(ApAddress::Address(format!(
                "{}/user/{}",
                server_url, profile.username
            ))),
            kind: ApActorType::Person,
            preferred_username: profile.username.clone(),
            inbox: format!("{}/user/{}/inbox", server_url, profile.username),
            outbox: format!("{}/user/{}/outbox", server_url, profile.username),
            followers: Some(format!(
                "{}/user/{}/followers",
                server_url, profile.username
            )),
            following: Some(format!(
                "{}/user/{}/following",
                server_url, profile.username
            )),
            subscribers: None,
            featured: None,
            featured_tags: None,
            manually_approves_followers: Some(false),
            published: {
                cfg_if::cfg_if! {
                    if #[cfg(feature = "pg")] {
                        Some(profile.created_at.to_rfc3339())
                    } else if #[cfg(feature = "sqlite")] {
                        use chrono::{DateTime, Utc};

                        Some(DateTime::<Utc>::from_naive_utc_and_offset(
                            profile.created_at,
                            Utc,
                        ).to_rfc3339())
                    }
                }
            },
            liked: Some(format!("{}/user/{}/liked", server_url, profile.username)),
            public_key: ApPublicKey {
                id: format!("{}/user/{}#main-key", server_url, profile.username),
                owner: format!("{}/user/{}", server_url, profile.username),
                public_key_pem: profile.public_key,
            },
            url: Some(format!("{}/@{}", server_url, profile.username)),
            icon: Some(ApImage {
                kind: ApImageType::Image,
                media_type: Some("image/png".to_string()),
                url: format!(
                    "{server_url}/{}",
                    profile
                        .avatar_filename
                        .unwrap_or((*crate::DEFAULT_AVATAR).clone())
                ),
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
            also_known_as: Some(vec![].into()),
            tag: Some(vec![]),
            endpoints: Some(ApEndpoint {
                shared_inbox: format!("{server_url}/inbox"),
            }),
            ephemeral_summary_markdown: None,
            ephemeral_following: None,
            ephemeral_leader_ap_id: None,
            ephemeral_followers: None,
            ephemeral_leaders: None,
            ephemeral_follow_activity_ap_id: None,
        }
    }
}

type ExtendedRemoteActor = (RemoteActor, Option<Leader>);

impl From<ExtendedRemoteActor> for ApActor {
    fn from((remote_actor, leader): ExtendedRemoteActor) -> Self {
        let mut actor = ApActor::from(remote_actor);

        actor.ephemeral_following = leader.clone().and_then(|x| x.accepted);

        actor.ephemeral_leader_ap_id = leader
            .clone()
            .and_then(|x| format!("{}/leader/{}", *crate::SERVER_URL, x.uuid).into());

        actor.ephemeral_follow_activity_ap_id = leader.and_then(|x| x.follow_ap_id);

        actor
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
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
                    subscribers: None,
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
                    ephemeral_summary_markdown: None,
                    ephemeral_followers: None,
                    ephemeral_leaders: None,
                    ephemeral_follow_activity_ap_id: None,
                }
            }
        }

        impl From<&RemoteActor> for ApActor {
            fn from(actor: &RemoteActor) -> Self {
                actor.clone().into()
            }
        }
    } else if #[cfg(feature = "sqlite")] {
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
                    subscribers: None,
                    liked: actor.liked,
                    public_key: serde_json::from_str(&actor.public_key).unwrap(),
                    featured: actor.featured,
                    featured_tags: actor.featured_tags,
                    url: actor.url,
                    manually_approves_followers: actor.manually_approves_followers,
                    published: actor.published,
                    tag: actor
                        .tag
                        .as_deref()
                        .and_then(|x| serde_json::from_str(x).ok()),
                    attachment: actor
                        .attachment
                        .as_deref()
                        .and_then(|x| serde_json::from_str(x).ok()),
                    endpoints: actor
                        .endpoints
                        .as_deref()
                        .and_then(|x| serde_json::from_str(x).ok()),
                    icon: actor
                        .icon
                        .as_deref()
                        .and_then(|x| serde_json::from_str(x).ok()),
                    image: actor
                        .image
                        .as_deref()
                        .and_then(|x| serde_json::from_str(x).ok()),
                    also_known_as: actor
                        .also_known_as
                        .as_deref()
                        .and_then(|x| serde_json::from_str(x).ok()),
                    discoverable: actor.discoverable,
                    capabilities: actor
                        .capabilities
                        .as_deref()
                        .and_then(|x| serde_json::from_str(x).ok()),
                    ephemeral_following: None,
                    ephemeral_leader_ap_id: None,
                    ephemeral_summary_markdown: None,
                    ephemeral_followers: None,
                    ephemeral_leaders: None,
                    ephemeral_follow_activity_ap_id: None,
                }
            }
        }
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
