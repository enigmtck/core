use crate::{models::actors::Actor, WEBFINGER_ACCT_RE};
use anyhow::Result;
use reqwest::Client;
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

impl WebFinger {
    pub fn get_address(&self) -> Option<String> {
        let captures = WEBFINGER_ACCT_RE.captures_iter(&self.subject).next()?;

        let username = captures.get(1)?.as_str();
        let domain = captures.get(2)?.as_str();

        Some(format!("@{username}@{domain}"))
    }

    pub fn get_id(&self) -> Option<String> {
        self.links
            .iter()
            .filter_map(|x| {
                if x.kind == Some("application/activity+json".to_string())
                || x.kind
                    == Some(
                        "application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\""
                            .to_string(),
                    )
            {
                x.href.clone()
            } else {
                None
            }
            })
            .take(1)
            .next()
    }
}

// This function is used in routes/webfinger.rs to create a struct for internal users.
// It's not useful to get a WebFinger for a remote Actor.
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
            ],
        }
    }
}

pub async fn retrieve_webfinger(domain: String, username: String) -> Result<WebFinger> {
    let url = format!("https://{domain}/.well-known/webfinger?resource=acct:{username}@{domain}");
    let accept = "application/jrd+json";
    let agent = "Enigmatick/0.1";

    let response = Client::builder()
        .user_agent(agent)
        .build()
        .unwrap()
        .get(url)
        .header("Accept", accept)
        .send()
        .await
        .map_err(anyhow::Error::msg)?;

    let json = response.json().await?;

    serde_json::from_value(json).map_err(anyhow::Error::msg)
}
