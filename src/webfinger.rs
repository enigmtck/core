use serde::{Deserialize, Serialize};

use crate::models::actors::Actor;

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

impl From<Actor> for WebFinger {
    fn from(profile: Actor) -> Self {
        let server_url = &*crate::SERVER_URL;
        let server_name = &*crate::SERVER_NAME;

        WebFinger {
            subject: format!(
                "acct:{}@{}",
                profile.ek_username.as_ref().unwrap(),
                server_name
            ),
            aliases: Some(vec![
                format!("{}/@{}", server_url, profile.ek_username.as_ref().unwrap()),
                format!(
                    "{}/user/{}",
                    server_url,
                    profile.ek_username.as_ref().unwrap()
                ),
            ]),
            links: vec![
                WebFingerLink {
                    rel: "http://webfinger.net/rel/profile-page".to_string(),
                    kind: Some("text/html".to_string()),
                    href: Some(format!(
                        "{}/@{}",
                        server_url,
                        profile.ek_username.as_ref().unwrap()
                    )),
                    ..Default::default()
                },
                WebFingerLink {
                    rel: "self".to_string(),
                    kind: Some("application/activity+json".to_string()),
                    href: Some(format!(
                        "{}/user/{}",
                        server_url,
                        profile.ek_username.unwrap()
                    )),
                    ..Default::default()
                },
                // WebFingerLink {
                //     rel: "self".to_string(),
                //     kind: Option::from(
                //         "application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\""
                //             .to_string(),
                //     ),
                //     href: Option::from(format!("{}/user/{}", server_url, profile.username)),
                //     ..Default::default()
                // },
                // WebFingerLink {
                //     rel: "http://ostatus.org/schema/1.0/subscribe".to_string(),
                //     kind: Option::None,
                //     href: Option::None,
                //     template: Option::from(format!(
                //         "{server_url}/authorize_interaction?uri={{uri}}"
                //     )),
                // },
            ],
        }
    }
}
