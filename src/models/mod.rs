use actors::Actor;
use chrono::{DateTime, Utc};
use objects::Object;
use serde::Serialize;
use serde_json::json;

pub mod activities;
pub mod actors;
pub mod cache;
pub mod encrypted_sessions;
pub mod followers;
pub mod instances;
pub mod leaders;
pub mod notifications;
pub mod objects;
pub mod olm_one_time_keys;
pub mod olm_sessions;
pub mod processing_queue;
pub mod profiles;
pub mod remote_encrypted_sessions;
pub mod unprocessable;
pub mod vault;

#[derive(Clone)]
pub enum Tombstone {
    Actor(Actor),
    Object(Object),
}

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub mod pg;

        use serde_json::Value;

        pub fn to_serde<T: Serialize>(object: &Option<T>) -> Option<Value> {
            object.as_ref().map(|x| json!(x))
        }

        pub fn from_serde<T: serde::de::DeserializeOwned>(object: Value) -> Option<T> {
            serde_json::from_value(object).ok()
        }

        pub fn from_serde_option<T: serde::de::DeserializeOwned>(object: Option<Value>) -> Option<T> {
            object.and_then(|o| serde_json::from_value(o).ok())
        }

        fn to_time(time: DateTime<Utc>) -> DateTime<Utc> {
            time
        }

        pub fn from_time(time: DateTime<Utc>) -> Option<DateTime<Utc>> {
             Some(time)
        }
    } else if #[cfg(feature = "sqlite")] {
        pub mod sqlite;

        pub fn to_serde<T: Serialize>(object: T) -> Option<String> {
            serde_json::to_string(&object).ok()
        }

        pub fn from_serde<T: serde::de::DeserializeOwned>(object: String) -> Option<T> {
            serde_json::from_str(&object).ok()
        }

        use chrono::NaiveDateTime;
        pub fn to_time(time: DateTime<Utc>) -> NaiveDateTime {
            time.naive_utc()
        }

        pub fn from_time(time: NaiveDateTime) -> Option<DateTime<Utc>> {
             Some(DateTime::<Utc>::from_naive_utc_and_offset(
                 time,
                 Utc,
             ))
        }
    }
}

pub struct OffsetPaging {
    pub page: u32,
    pub limit: u32,
}
