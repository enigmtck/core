use crate::activity_pub::{ApNote, ApTag};
use crate::db::Db;
use crate::schema::note_hashtags;
use crate::POOL;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

use super::notes::Note;

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

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = note_hashtags)]
pub struct NoteHashtag {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub hashtag: String,
    pub note_id: i32,
}

pub async fn create_note_hashtag(
    conn: Option<&Db>,
    hashtag: NewNoteHashtag,
) -> Option<NoteHashtag> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(note_hashtags::table)
                    .values(&hashtag)
                    .get_result::<NoteHashtag>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(note_hashtags::table)
                .values(&hashtag)
                .get_result::<NoteHashtag>(&mut pool)
                .ok()
        }
    }
}
