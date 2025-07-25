use serde::{Deserialize, Serialize};

use crate::{DOMAIN_RE, LOCAL_RE, LOCAL_URL_RE, WEBFINGER_RE};

pub fn is_local(ap_id: String) -> bool {
    if LOCAL_RE.is_match(&ap_id) {
        log::debug!("looks local");
        true
    } else {
        log::debug!("looks remote");
        false
    }
}

pub fn get_domain_from_url(url: String) -> Option<String> {
    Some(DOMAIN_RE.captures(&url)?[1].to_string())
}

pub fn get_domain_from_webfinger(webfinger: String) -> String {
    WEBFINGER_RE
        .captures(&webfinger)
        .expect("unable to locate domain name")[2]
        .to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub enum LocalIdentifierType {
    User,
    Note,
    Session,
    Collection,
    Activity,
    #[default]
    None,
}

impl From<&str> for LocalIdentifierType {
    fn from(text: &str) -> Self {
        match text {
            "user" => LocalIdentifierType::User,
            "notes" => LocalIdentifierType::Note,
            "session" => LocalIdentifierType::Session,
            "collections" => LocalIdentifierType::Collection,
            "activities" => LocalIdentifierType::Activity,
            _ => LocalIdentifierType::None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LocalIdentifier {
    pub identifier: String,
    #[serde(rename = "type")]
    pub kind: LocalIdentifierType,
}

pub fn get_local_identifier(ap_id: String) -> Option<LocalIdentifier> {
    if let Some(ap_id_match) = LOCAL_URL_RE.captures(&ap_id) {
        log::debug!("IDENTIFIER MATCH: {ap_id_match:#?}");
        Some(LocalIdentifier {
            identifier: ap_id_match.get(2).unwrap().as_str().to_string(),
            kind: ap_id_match.get(1).unwrap().as_str().into(),
        })
    } else {
        None
    }
}

pub fn get_session_as_id_from_uuid(uuid: String) -> String {
    format!("https://{}/sessions/{uuid}", *crate::SERVER_NAME)
}

pub fn get_instrument_as_id_from_uuid(uuid: String) -> String {
    format!("https://{}/instruments/{uuid}", *crate::SERVER_NAME)
}

pub fn get_followers_ap_id_from_username(username: String) -> String {
    format!("https://{}/user/{username}/followers", *crate::SERVER_NAME)
}

pub fn get_following_ap_id_from_username(username: String) -> String {
    format!("https://{}/user/{username}/following", *crate::SERVER_NAME)
}

pub fn get_ap_id_from_username(username: String) -> String {
    format!("https://{}/user/{username}", *crate::SERVER_NAME)
}

pub fn get_object_ap_id_from_uuid(uuid: String) -> String {
    format!("https://{}/objects/{uuid}", *crate::SERVER_NAME)
}

pub fn get_conversation_ap_id_from_uuid(uuid: String) -> String {
    format!("https://{}/conversations/{uuid}", *crate::SERVER_NAME)
}

pub fn get_object_url_from_uuid(uuid: String) -> String {
    format!("https://{}/objects?uuid={uuid}", *crate::SERVER_NAME)
}

pub fn get_activity_ap_id_from_uuid(uuid: String) -> String {
    format!("https://{}/activities/{uuid}", *crate::SERVER_NAME)
}

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        use serde_json::Value;

        pub fn handle_option(v: Value) -> Option<Value> {
            if v == Value::Null {
                None
            } else {
                Some(v)
            }
        }
    } else if #[cfg(feature = "sqlite")] {
        pub fn handle_option(v: String) -> Option<String> {
            if String::is_empty(&v) {
                None
            } else {
                Some(v)
            }
        }

    }
}

pub fn get_uuid(id: String) -> Option<String> {
    let identifier = get_local_identifier(id)?;

    if identifier.kind == LocalIdentifierType::Activity {
        Some(identifier.identifier)
    } else {
        None
    }
}
