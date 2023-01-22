use crate::activity_pub::{
    ApActorType, ApAttachment, ApContext, ApEndpoint, ApImage, ApImageType, ApTag,
};
use crate::models::followers::Follower;
use crate::models::leaders::Leader;
use crate::models::profiles::Profile;
use crate::models::remote_actors::RemoteActor;
use crate::schema::profiles::banner_filename;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

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
    accepts_chat_messages: bool,
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
    pub id: Option<String>,
    pub preferred_username: String,
    pub inbox: String,
    pub outbox: String,
    pub followers: String,
    pub following: String,
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
            context: Option::from(ApContext::Plain(
                "https://www.w3.org/ns/activitystreams".to_string(),
            )),
            kind: ApActorType::default(),
            name: Option::None,
            summary: Option::None,
            id: Option::None,
            preferred_username: String::new(),
            inbox: String::new(),
            outbox: String::new(),
            followers: String::new(),
            following: String::new(),
            liked: Option::None,
            public_key: ApPublicKey::default(),
            featured: Option::None,
            featured_tags: Option::None,
            url: Option::None,
            manually_approves_followers: Option::None,
            published: Option::None,
            tag: Option::None,
            attachment: Option::None,
            endpoints: Option::None,
            icon: Option::None,
            image: Option::None,
            also_known_as: Option::None,
            discoverable: Option::None,
            capabilities: Option::None,
            ephemeral_following: Option::None,
            ephemeral_leader_ap_id: Option::None,
        }
    }
}

impl From<Profile> for ApActor {
    fn from(profile: Profile) -> Self {
        let server_url = &*crate::SERVER_URL;

        ApActor {
            name: Option::from(profile.display_name),
            summary: Option::from(profile.summary.unwrap_or_default()),
            id: Option::from(format!("{}/user/{}", server_url, profile.username)),
            kind: ApActorType::Person,
            preferred_username: profile.username.clone(),
            inbox: format!("{}/user/{}/inbox/", server_url, profile.username),
            outbox: format!("{}/user/{}/outbox/", server_url, profile.username),
            followers: format!("{}/user/{}/followers/", server_url, profile.username),
            following: format!("{}/user/{}/following/", server_url, profile.username),
            liked: Option::from(format!("{}/user/{}/liked/", server_url, profile.username)),
            public_key: ApPublicKey {
                id: format!("{}/user/{}#main-key", server_url, profile.username),
                owner: format!("{}/user/{}", server_url, profile.username),
                public_key_pem: profile.public_key,
            },
            url: Option::from(format!("{}/@{}", server_url, profile.username)),
            icon: Option::from(ApImage {
                kind: ApImageType::Image,
                media_type: Option::None,
                url: format!("{}/{}", server_url, profile.avatar_filename),
            }),
            image: {
                if let Some(banner) = profile.banner_filename {
                    Option::from(ApImage {
                        kind: ApImageType::Image,
                        media_type: Option::None,
                        url: format!("{}/{}", server_url, banner),
                    })
                } else {
                    Option::None
                }
            },
            ..Default::default()
        }
    }
}

impl From<RemoteActor> for ApActor {
    fn from(actor: RemoteActor) -> Self {
        ApActor {
            context: Option::from(ApContext::Plain(
                "https://www.w3.org/ns/activitystreams".to_string(),
            )),
            kind: ApActorType::Person,
            name: Option::from(actor.name),
            summary: actor.summary,
            id: Option::from(actor.ap_id),
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
            ephemeral_following: Option::None,
            ephemeral_leader_ap_id: Option::None,
        }
    }
}

type RemoteActorAndLeader = (RemoteActor, Option<Leader>);

impl From<RemoteActorAndLeader> for ApActor {
    fn from(actor_and_leader: RemoteActorAndLeader) -> Self {
        let mut actor: ApActor = actor_and_leader.0.into();

        actor.ephemeral_following = {
            if let Some(leader) = actor_and_leader.1.clone() {
                leader.accepted
            } else {
                Option::None
            }
        };

        actor.ephemeral_leader_ap_id = {
            if let Some(leader) = actor_and_leader.1 {
                format!("{}/leader/{}", *crate::SERVER_URL, leader.uuid).into()
            } else {
                Option::None
            }
        };

        actor
    }
}
