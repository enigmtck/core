use crate::db::Db;
use crate::models::remote_note_hashtags::NewRemoteNoteHashtag;
use crate::schema::remote_note_hashtags;
use crate::POOL;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::Serialize;

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
