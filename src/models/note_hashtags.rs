use crate::activity_pub::{ApNote, ApTag};
use crate::schema::note_hashtags;
use diesel::Insertable;
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

use crate::models::notes::Note;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::note_hashtags::NoteHashtag;
        pub use crate::models::pg::note_hashtags::create_note_hashtag;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::note_hashtags::NoteHashtag;
        pub use crate::models::sqlite::note_hashtags::create_note_hashtag;
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = note_hashtags)]
pub struct NewNoteHashtag {
    pub hashtag: String,
    pub note_id: i32,
}

impl From<Note> for Vec<NewNoteHashtag> {
    fn from(note: Note) -> Self {
        let ap_note: ApNote = note.clone().into();

        ap_note
            .tag
            .unwrap_or_default()
            .iter()
            .filter_map(|tag| {
                if let ApTag::HashTag(tag) = tag {
                    Some(NewNoteHashtag {
                        hashtag: tag.name.clone(),
                        note_id: note.id,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}
