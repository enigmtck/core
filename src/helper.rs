use serde::{Deserialize, Serialize};
use serde_json::Value;

pub fn is_local(ap_id: String) -> bool {
    let pattern = format!(r#"\w+?://{}/(.+)"#, *crate::SERVER_NAME);

    if let Ok(re) = regex::Regex::new(&pattern) {
        if re.is_match(&ap_id) {
            log::debug!("looks local");
            true
        } else {
            log::debug!("looks remote");
            false
        }
    } else {
        false
    }
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
    let pattern = format!(
        r#"^{}/(user|notes|session|collections|activities)/(.+)$"#,
        *crate::SERVER_URL
    );

    if let Ok(re) = regex::Regex::new(&pattern) {
        if let Some(ap_id_match) = re.captures(&ap_id) {
            log::debug!("IDENTIFIER MATCH: {ap_id_match:#?}");
            Some(LocalIdentifier {
                identifier: ap_id_match.get(2).unwrap().as_str().to_string(),
                kind: ap_id_match.get(1).unwrap().as_str().into(),
            })
        } else {
            None
        }
    } else {
        None
    }
}

pub fn get_ap_id_from_username(username: String) -> String {
    format!("https://{}/user/{}", *crate::SERVER_NAME, username)
}

pub fn get_note_ap_id_from_uuid(uuid: String) -> String {
    format!("https://{}/notes/{}", *crate::SERVER_NAME, uuid)
}

pub fn get_activity_ap_id_from_uuid(uuid: String) -> String {
    format!("https://{}/activities/{}", *crate::SERVER_NAME, uuid)
}

pub fn handle_option(v: Value) -> Option<Value> {
    if v == Value::Null {
        None
    } else {
        Some(v)
    }
}
