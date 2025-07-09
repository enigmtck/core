use std::fmt::Display;

use super::Inbox;
use rocket::data::{self, Data, FromData, ToByteUnit};
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::Request;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::signing::get_hash;
use std::fmt;
use urlencoding::encode;

pub mod accept;
pub mod add;
pub mod announce;
pub mod ap_move;
pub mod block;
pub mod create;
pub mod delete;
pub mod follow;
pub mod like;
pub mod remove;
pub mod undo;
pub mod update;

pub fn sanitize_json_fields(mut value: Value) -> Value {
    if let Value::Object(ref mut obj) = value {
        // Handle top level: remove "attributedTo" if both exist and are identical
        sanitize_level(obj, "actor", "attributedTo");

        // Handle conversation/context overlap at top level
        sanitize_level(obj, "conversation", "context");

        // Handle one level deeper in "object" field
        if let Some(Value::Object(ref mut object_obj)) = obj.get_mut("object") {
            // In object level: remove "actor" if both exist and are identical
            sanitize_level(object_obj, "attributedTo", "actor");

            // Handle conversation/context overlap in object level
            sanitize_level(object_obj, "conversation", "context");
        }
    }
    value
}

fn sanitize_level(obj: &mut Map<String, Value>, keep_field: &str, remove_field: &str) {
    if let (Some(keep_val), Some(remove_val)) = (obj.get(keep_field), obj.get(remove_field)) {
        // If both are identical, remove the unwanted field
        if keep_val == remove_val {
            obj.remove(remove_field);
        }
        // If one is null, consolidate to the desired survivor
        else if keep_val.is_null() && !remove_val.is_null() {
            obj.insert(keep_field.to_string(), remove_val.clone());
            obj.remove(remove_field);
        }
        // If remove_val is null (regardless of keep_val), remove the unwanted field
        else if remove_val.is_null() {
            obj.remove(remove_field);
        }
        // If they're different and neither is null, log warning and remove unwanted field
        else {
            log::warn!(
                "Mismatch between {keep_field} and {remove_field}: {keep_val} vs {remove_val}"
            );
            obj.remove(remove_field);
        }
    }
    // If only one exists or neither exists - no action needed
}

#[derive(FromFormField, Eq, PartialEq, Debug, Clone, Deserialize)]
pub enum InboxView {
    #[serde(alias = "home")]
    Home,
    #[serde(alias = "local")]
    Local,
    #[serde(alias = "global")]
    Global,
    #[serde(alias = "direct")]
    Direct,
}

impl Display for InboxView {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:#?}")
    }
}

pub struct HashedJson {
    pub hash: String,
    pub json: Value,
}

#[rocket::async_trait]
impl<'r> FromData<'r> for HashedJson {
    type Error = anyhow::Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> data::Outcome<'r, Self> {
        let limit = req.limits().get("json").unwrap_or(1.mebibytes());

        let bytes = match data.open(limit).into_bytes().await {
            Ok(bytes) if bytes.is_complete() => bytes.into_inner(),
            Ok(_) => {
                return Outcome::Error((
                    Status::PayloadTooLarge,
                    anyhow::anyhow!("JSON POST too large"),
                ))
            }
            Err(e) => {
                return Outcome::Error((
                    Status::InternalServerError,
                    anyhow::anyhow!("IO error: {}", e),
                ))
            }
        };

        let hash = get_hash(bytes.clone());
        let json = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(e) => {
                return Outcome::Error((Status::BadRequest, anyhow::anyhow!("Invalid JSON: {}", e)))
            }
        };

        Outcome::Success(HashedJson { hash, json })
    }
}

pub fn convert_hashtags_to_query_string(hashtags: &[String]) -> String {
    hashtags
        .iter()
        .map(|tag| format!("&hashtags[]={}", encode(tag)))
        .collect::<Vec<String>>()
        .join("")
}

pub fn add_hash_to_tags(hashtags: &[String]) -> Vec<String> {
    hashtags
        .iter()
        .map(|tag| format!("#{tag}"))
        .collect::<Vec<String>>()
}
