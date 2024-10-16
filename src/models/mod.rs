use chrono::{DateTime, Utc};
use serde::Serialize;

pub mod activities;
pub mod actors;
pub mod cache;
pub mod encrypted_sessions;
pub mod followers;
pub mod instances;
pub mod leaders;
pub mod note_hashtags;
pub mod notes;
pub mod notifications;
pub mod objects;
pub mod olm_one_time_keys;
pub mod olm_sessions;
pub mod processing_queue;
//pub mod profiles;
pub mod remote_actor_hashtags;
pub mod remote_actors;
pub mod remote_encrypted_sessions;
//pub mod remote_note_hashtags;
//pub mod remote_notes;
//pub mod remote_questions;
pub mod vault;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub mod pg;

        use serde_json::Value;
        pub fn to_serde<T: Serialize>(object: T) -> Option<Value> {
            serde_json::to_value(object).ok()
        }

        pub fn from_serde<T: serde::de::DeserializeOwned>(object: Value) -> Option<T> {
            serde_json::from_value(object).ok()
        }

        pub fn from_serde_option<T: serde::de::DeserializeOwned>(object: Option<Value>) -> Option<T> {
            object.map(|o| serde_json::from_value(o).ok()).flatten()
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
