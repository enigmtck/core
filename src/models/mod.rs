use serde::{de::DeserializeOwned, Serialize};

pub mod activities;
pub mod cache;
pub mod encrypted_sessions;
pub mod followers;
pub mod instances;
pub mod leaders;
pub mod note_hashtags;
pub mod notes;
pub mod notifications;
pub mod olm_one_time_keys;
pub mod olm_sessions;
pub mod processing_queue;
pub mod profiles;
pub mod remote_actor_hashtags;
pub mod remote_actors;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use pg::remote_encrypted_sessions;
        pub use pg::remote_note_hashtags;
        pub use pg::remote_notes;
        pub use pg::timeline;
        pub use pg::timeline_hashtags;
        pub use pg::vault;
        pub mod pg;
    } else if #[cfg(feature = "sqlite")] {
        pub use sqlite::remote_encrypted_sessions;
        pub use sqlite::remote_note_hashtags;
        pub use sqlite::remote_notes;
        pub use sqlite::timeline;
        pub use sqlite::timeline_hashtags;
        pub use sqlite::vault;
        pub mod sqlite;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        use serde_json::Value;
        pub fn to_serde<T: Serialize>(object: T) -> Option<Value> {
            serde_json::to_value(object).ok()
        }
        pub fn from_serde<T>(object: Value) -> Option<T> {
            serde_json::from_value(object).ok()
        }
    } else if #[cfg(feature = "sqlite")] {
        pub fn to_serde<T: Serialize>(object: T) -> Option<String> {
            serde_json::to_string(&object).ok()
        }
        pub fn from_serde<T: DeserializeOwned>(object: String) -> Option<T> {
            serde_json::from_str(&object).ok()
        }
    }
}
