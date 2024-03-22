use crate::db::Db;
use crate::schema::remote_notes;
use crate::POOL;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Insertable, Default, Debug, AsChangeset)]
#[diesel(table_name = remote_notes)]
pub struct NewRemoteNote {
    pub kind: String,
    pub ap_id: String,
    pub published: Option<String>,
    pub url: Option<String>,
    pub attributed_to: Option<String>,
    pub ap_to: Option<String>,
    pub cc: Option<String>,
    pub content: String,
    pub attachment: Option<String>,
    pub tag: Option<String>,
    pub replies: Option<String>,
    pub signature: Option<String>,
    pub summary: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub atom_uri: Option<String>,
    pub in_reply_to: Option<String>,
    pub in_reply_to_atom_uri: Option<String>,
    pub conversation: Option<String>,
    pub content_map: Option<String>,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Default, Debug)]
#[diesel(table_name = remote_notes)]
pub struct RemoteNote {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub kind: String,
    pub ap_id: String,
    pub published: Option<String>,
    pub url: Option<String>,
    pub ap_to: Option<String>,
    pub cc: Option<String>,
    pub tag: Option<String>,
    pub attributed_to: String,
    pub content: String,
    pub attachment: Option<String>,
    pub replies: Option<String>,
    pub in_reply_to: Option<String>,
    pub signature: Option<String>,
    pub summary: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub atom_uri: Option<String>,
    pub in_reply_to_atom_uri: Option<String>,
    pub conversation: Option<String>,
    pub content_map: Option<String>,
}

pub async fn create_or_update_remote_note(
    conn: Option<&Db>,
    note: NewRemoteNote,
) -> Option<RemoteNote> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(remote_notes::table)
                    .values(&note)
                    .on_conflict(remote_notes::ap_id)
                    .do_update()
                    .set(&note)
                    .execute(c)
                    .ok()?;

                remote_notes::table
                    .filter(remote_notes::ap_id.eq(&note.ap_id))
                    .first::<RemoteNote>(c)
                    .ok()
            })
            .await
        }
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(remote_notes::table)
                .values(&note)
                .on_conflict(remote_notes::ap_id)
                .do_update()
                .set(&note)
                .execute(&mut pool)
                .ok()?;

            remote_notes::table
                .filter(remote_notes::ap_id.eq(&note.ap_id))
                .first::<RemoteNote>(&mut pool)
                .ok()
        }
    }
}
