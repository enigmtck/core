use crate::db::Db;
use crate::schema::remote_notes;
use crate::POOL;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::notes::NoteType;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, AsChangeset)]
#[diesel(table_name = remote_notes)]
pub struct NewRemoteNote {
    pub kind: NoteType,
    pub ap_id: String,
    pub published: Option<String>,
    pub url: Option<String>,
    pub attributed_to: Option<String>,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub content: String,
    pub attachment: Option<Value>,
    pub tag: Option<Value>,
    pub replies: Option<Value>,
    pub signature: Option<Value>,
    pub summary: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub atom_uri: Option<String>,
    pub in_reply_to: Option<String>,
    pub in_reply_to_atom_uri: Option<String>,
    pub conversation: Option<String>,
    pub content_map: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Default, Debug)]
#[diesel(table_name = remote_notes)]
pub struct RemoteNote {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub kind: NoteType,
    pub ap_id: String,
    pub published: Option<String>,
    pub url: Option<String>,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub tag: Option<Value>,
    pub attributed_to: String,
    pub content: String,
    pub attachment: Option<Value>,
    pub replies: Option<Value>,
    pub in_reply_to: Option<String>,
    pub signature: Option<Value>,
    pub summary: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub atom_uri: Option<String>,
    pub in_reply_to_atom_uri: Option<String>,
    pub conversation: Option<String>,
    pub content_map: Option<Value>,
    pub metadata: Option<Value>,
}

pub async fn update_metadata(
    conn: Option<&Db>,
    id: i32,
    metadata: Value,
) -> Result<RemoteNote, anyhow::Error> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::update(remote_notes::table.filter(remote_notes::id.eq(id)))
                    .set(remote_notes::metadata.eq(Some(metadata)))
                    .get_result::<RemoteNote>(c)
            })
            .await
            .map_err(anyhow::Error::msg),
        None => {
            let mut pool = POOL.get()?;
            diesel::update(remote_notes::table.filter(remote_notes::id.eq(id)))
                .set(remote_notes::metadata.eq(Some(metadata)))
                .get_result::<RemoteNote>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}

pub async fn create_or_update_remote_note(
    conn: Option<&Db>,
    note: NewRemoteNote,
) -> Option<RemoteNote> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(remote_notes::table)
                    .values(&note)
                    .on_conflict(remote_notes::ap_id)
                    .do_update()
                    .set(&note)
                    .get_result::<RemoteNote>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(remote_notes::table)
                .values(&note)
                .on_conflict(remote_notes::ap_id)
                .do_update()
                .set(&note)
                .get_result::<RemoteNote>(&mut pool)
                .ok()
        }
    }
}
