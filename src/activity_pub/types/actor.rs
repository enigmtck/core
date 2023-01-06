use crate::activity_pub::{ApActorType, ApContext};
use crate::models::profiles::Profile;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApPublicKey {
    pub id: String,
    pub owner: String,
    pub public_key_pem: String,
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
    pub name: Option<String>,
    pub summary: Option<String>,
    pub id: Option<String>,
    pub preferred_username: String,
    pub inbox: String,
    pub outbox: String,
    pub followers: String,
    pub following: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liked: Option<String>,
    pub public_key: ApPublicKey,
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
            ..Default::default()
        }
    }
}
