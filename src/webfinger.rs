use crate::models::profiles::Profile;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct WebFingerLink {
    pub rel: String,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct WebFinger {
    pub subject: String,
    pub aliases: Option<Vec<String>>,
    pub links: Vec<WebFingerLink>,
}

impl From<Profile> for WebFinger {
    fn from(profile: Profile) -> Self {
        let server_url = &*crate::SERVER_URL;
        let server_name = &*crate::SERVER_NAME;

        WebFinger {
            subject: format!("acct:{}@{}", profile.username, server_name),
            aliases: Option::from(vec![
                format!("{}/@{}", server_url, profile.username),
                format!("{}/users/{}", server_url, profile.username),
            ]),
            links: vec![
                WebFingerLink {
                    rel: "http://webfinger.net/rel/profile-page".to_string(),
                    kind: Option::from("text/html".to_string()),
                    href: Option::from(format!("{}/@{}", server_url, profile.username)),
                    ..Default::default()
                },
                WebFingerLink {
                    rel: "self".to_string(),
                    kind: Option::from("application/activity+json".to_string()),
                    href: Option::from(format!("{}/user/{}", server_url, profile.username)),
                    ..Default::default()
                },
            ],
        }
    }
}
