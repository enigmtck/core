use crate::activity_pub::{ApNote, ApTag};
use crate::schema::remote_note_hashtags;
use diesel::Insertable;
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

use super::remote_notes::RemoteNote;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::remote_note_hashtags::RemoteNoteHashtag;
        pub use crate::models::pg::remote_note_hashtags::create_remote_note_hashtag;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::remote_note_hashtags::RemoteNoteHashtag;
        pub use crate::models::sqlite::remote_note_hashtags::create_remote_note_hashtag;
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = remote_note_hashtags)]
pub struct NewRemoteNoteHashtag {
    pub hashtag: String,
    pub remote_note_id: i32,
}

impl From<RemoteNote> for Vec<NewRemoteNoteHashtag> {
    fn from(remote_note: RemoteNote) -> Self {
        let ap_note: ApNote = remote_note.clone().into();

        ap_note
            .tag
            .unwrap_or_default()
            .iter()
            .filter_map(|tag| {
                if let ApTag::HashTag(tag) = tag {
                    Some(NewRemoteNoteHashtag {
                        hashtag: tag.name.clone(),
                        remote_note_id: remote_note.id,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}
