use serde::Serialize;

pub mod activities;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use pg::cache;
        pub use pg::encrypted_sessions;
        pub use pg::followers;
        pub use pg::instances;
        pub use pg::leaders;
        pub use pg::note_hashtags;
        pub use pg::notes;
        pub use pg::notifications;
        pub use pg::olm_one_time_keys;
        pub use pg::olm_sessions;
        pub use pg::processing_queue;
        pub use pg::profiles;
        pub use pg::remote_actor_hashtags;
        pub use pg::remote_actors;
        pub use pg::remote_encrypted_sessions;
        pub use pg::remote_note_hashtags;
        pub use pg::remote_notes;
        pub use pg::timeline;
        pub use pg::timeline_hashtags;
        pub use pg::vault;
        pub mod pg;
    } else if #[cfg(feature = "sqlite")] {
        pub use sqlite::cache;
        pub use sqlite::encrypted_sessions;
        pub use sqlite::followers;
        pub use sqlite::instances;
        pub use sqlite::leaders;
        pub use sqlite::note_hashtags;
        pub use sqlite::notes;
        pub use sqlite::notifications;
        pub use sqlite::olm_one_time_keys;
        pub use sqlite::olm_sessions;
        pub use sqlite::processing_queue;
        pub use sqlite::profiles;
        pub use sqlite::remote_actor_hashtags;
        pub use sqlite::remote_actors;
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
    } else if #[cfg(feature = "sqlite")] {
        pub fn to_serde<T: Serialize>(object: T) -> Option<String> {
            serde_json::to_string(&object).ok()
        }
    }
}
