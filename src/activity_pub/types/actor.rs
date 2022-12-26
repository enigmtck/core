use crate::activity_pub::{ApActorType, ApBaseObject, ApContext};
use crate::models::profiles::Profile;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    #[serde(flatten)]
    pub base: ApBaseObject,
    #[serde(rename = "type")]
    pub kind: ApActorType,
    pub preferred_username: String,
    pub inbox: String,
    pub outbox: String,
    pub followers: String,
    pub following: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liked: Option<String>,
    pub public_key: ApPublicKey,
}

impl From<Profile> for ApActor {
    fn from(profile: Profile) -> Self {
        let server_url = &*crate::SERVER_URL;

        ApActor {
            base: ApBaseObject {
                context: Option::from(ApContext::Plain(
                    "https://www.w3.org/ns/activitystreams".to_string(),
                )),
                name: Option::from(profile.display_name),
                summary: Option::from(profile.summary.unwrap_or_default()),
                id: Option::from(format!("{}/user/{}", server_url, profile.username)),
                ..Default::default()
            },
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
        }
    }
}
