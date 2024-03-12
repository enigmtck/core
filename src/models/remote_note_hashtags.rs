use crate::activity_pub::{ApNote, ApTag};
use crate::db::Db;
use crate::schema::remote_note_hashtags;
use crate::POOL;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

use super::remote_notes::RemoteNote;

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

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = remote_note_hashtags)]
pub struct RemoteNoteHashtag {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub hashtag: String,
    pub remote_note_id: i32,
}

pub async fn create_remote_note_hashtag(
    conn: Option<&Db>,
    hashtag: NewRemoteNoteHashtag,
) -> Option<RemoteNoteHashtag> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(remote_note_hashtags::table)
                    .values(&hashtag)
                    .execute(c)
                    .ok()?;

                remote_note_hashtags::table
                    .order(remote_note_hashtags::id.desc())
                    .first::<RemoteNoteHashtag>(c)
                    .ok()
            })
            .await
        }
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(remote_note_hashtags::table)
                .values(&hashtag)
                .execute(&mut pool)
                .ok()?;

            remote_note_hashtags::table
                .order(remote_note_hashtags::id.desc())
                .first::<RemoteNoteHashtag>(&mut pool)
                .ok()
        }
    }
}
