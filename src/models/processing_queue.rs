use crate::activity_pub::{ApNote, ApSession};
use crate::db::Db;
use crate::schema::processing_queue;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::remote_encrypted_sessions::RemoteEncryptedSession;
use super::remote_notes::RemoteNote;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[table_name = "processing_queue"]
pub struct NewProcessingItem {
    pub profile_id: i32,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: Value,
    pub cc: Option<Value>,
    pub attributed_to: String,
    pub ap_object: Value,
    pub processed: bool,
}

impl From<RemoteNote> for NewProcessingItem {
    fn from(note: RemoteNote) -> Self {
        let ap_note: ApNote = note.clone().into();

        NewProcessingItem {
            profile_id: note.profile_id,
            kind: note.clone().kind,
            ap_id: format!("{}#processing", note.ap_id),
            ap_to: note.clone().ap_to.unwrap(),
            attributed_to: note.clone().attributed_to,
            cc: note.cc,
            ap_object: serde_json::to_value(ap_note).unwrap(),
            processed: false,
        }
    }
}

impl From<RemoteEncryptedSession> for NewProcessingItem {
    fn from(session: RemoteEncryptedSession) -> Self {
        let ap_session: ApSession = session.clone().into();

        NewProcessingItem {
            profile_id: session.profile_id,
            kind: session.clone().kind,
            ap_id: format!("{}#processing", session.ap_id),
            ap_to: serde_json::to_value(&session.ap_to).unwrap(),
            attributed_to: session.attributed_to,
            cc: Option::None,
            ap_object: serde_json::to_value(ap_session).unwrap(),
            processed: false,
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "processing_queue"]
pub struct ProcessingItem {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile_id: i32,
    pub ap_id: String,
    pub ap_to: Value,
    pub cc: Option<Value>,
    pub attributed_to: String,
    pub kind: String,
    pub ap_object: Value,
    pub processed: bool,
}

pub async fn create_processing_item(
    conn: &Db,
    processing_item: NewProcessingItem,
) -> Option<ProcessingItem> {
    match conn
        .run(move |c| {
            diesel::insert_into(processing_queue::table)
                .values(&processing_item)
                .get_result::<ProcessingItem>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}

pub async fn get_unprocessed_items_by_profile_id(conn: &Db, id: i32) -> Vec<ProcessingItem> {
    match conn
        .run(move |c| {
            let query = processing_queue::table
                .filter(processing_queue::profile_id.eq(id))
                .filter(processing_queue::processed.eq(false))
                .order(processing_queue::created_at.asc())
                .into_boxed();

            query.get_results::<ProcessingItem>(c)
        })
        .await
    {
        Ok(x) => x,
        Err(_) => vec![],
    }
}