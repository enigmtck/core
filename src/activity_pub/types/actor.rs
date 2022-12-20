use crate::models::profiles::Profile;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DefaultOnError};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MastodonContextBasic {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "@type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(rename = "@container")]
    #[serde(skip_serializing_if = "Option::is_none")]
    container: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MastodonContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    manually_approves_followers: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    toot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    featured: Option<MastodonContextBasic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    featured_tags: Option<MastodonContextBasic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    also_known_as: Option<MastodonContextBasic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    moved_to: Option<MastodonContextBasic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    discoverable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    claim: Option<MastodonContextBasic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fingerprint_key: Option<MastodonContextBasic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    identity_key: Option<MastodonContextBasic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    devices: Option<MastodonContextBasic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_franking: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cipher_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suspended: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focal_point: Option<MastodonContextBasic>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Context {
    Plain(String),
    Complex(Box<MastodonContext>),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PublicKey {
    pub id: String,
    pub owner: String,
    pub public_key_pem: String,
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    #[serde(rename = "@context")]
    #[serde_as(deserialize_as = "DefaultOnError")]
    pub context: Vec<Context>,
    #[serde(rename = "type")]
    pub kind: String,
    pub id: String,
    pub name: String,
    pub preferred_username: String,
    pub summary: String,
    pub inbox: String,
    pub outbox: String,
    pub followers: String,
    pub following: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liked: Option<String>,
    pub public_key: PublicKey,
}

impl From<Profile> for Actor {
    fn from(profile: Profile) -> Self {
        let server_url = &*crate::SERVER_URL;

        Actor {
            context: vec![Context::Plain(
                "https://www.w3.org/ns/activitystreams".to_string(),
            )],
            kind: "Person".to_string(),
            id: format!("{}/user/{}", server_url, profile.username),
            name: profile.display_name,
            preferred_username: profile.username.clone(),
            summary: profile.summary.unwrap_or_default(),
            inbox: format!("{}/user/{}/inbox/", server_url, profile.username),
            outbox: format!("{}/user/{}/outbox/", server_url, profile.username),
            followers: format!("{}/user/{}/followers/", server_url, profile.username),
            following: format!("{}/user/{}/following/", server_url, profile.username),
            liked: Option::from(format!("{}/user/{}/liked/", server_url, profile.username)),
            public_key: PublicKey {
                id: format!("{}/user/{}#main-key", server_url, profile.username),
                owner: format!("{}/user/{}", server_url, profile.username),
                public_key_pem: profile.public_key,
            },
        }
    }
}
