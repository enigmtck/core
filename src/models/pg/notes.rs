use crate::activity_pub::ApNoteType;
use crate::db::Db;
use crate::schema::notes;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(
    diesel_derive_enum::DbEnum, Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq,
)]
#[ExistingTypePath = "crate::schema::sql_types::NoteType"]
pub enum NoteType {
    #[default]
    Note,
    EncryptedNote,
    VaultNote,
    Question,
}

impl fmt::Display for NoteType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<ApNoteType> for NoteType {
    fn from(kind: ApNoteType) -> Self {
        match kind {
            ApNoteType::EncryptedNote => NoteType::EncryptedNote,
            ApNoteType::Note => NoteType::Note,
            ApNoteType::VaultNote => NoteType::VaultNote,
            ApNoteType::Question => NoteType::Question,
        }
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = notes)]
pub struct NewNote {
    pub uuid: String,
    pub profile_id: i32,
    pub content: String,
    pub kind: NoteType,
    pub ap_to: Value,
    pub attributed_to: String,
    pub in_reply_to: Option<String>,
    pub tag: Option<Value>,
    pub attachment: Option<Value>,
    pub cc: Option<Value>,
    pub conversation: Option<String>,
    pub instrument: Option<Value>,
    pub ap_id: Option<String>,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = notes)]
pub struct Note {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub profile_id: i32,
    pub kind: NoteType,
    pub ap_to: Value,
    pub cc: Option<Value>,
    pub tag: Option<Value>,
    pub attributed_to: String,
    pub in_reply_to: Option<String>,
    pub content: String,
    pub conversation: Option<String>,
    pub attachment: Option<Value>,
    pub instrument: Option<Value>,
    pub ap_id: Option<String>,
}

pub async fn create_note(conn: &Db, note: NewNote) -> Option<Note> {
    conn.run(move |c| {
        diesel::insert_into(notes::table)
            .values(&note)
            .get_result::<Note>(c)
    })
    .await
    .ok()
}

pub async fn get_notes_by_profile_id(
    conn: &Db,
    profile_id: i32,
    limit: i64,
    offset: i64,
    exclude_replies: bool,
) -> Vec<Note> {
    conn.run(move |c| {
        let mut query = notes::table
            .filter(notes::profile_id.eq(profile_id))
            .filter(notes::kind.eq(NoteType::Note))
            .order(notes::created_at.desc())
            .limit(limit)
            .offset(offset)
            .into_boxed();

        if exclude_replies {
            query = query.filter(notes::in_reply_to.is_null());
        }

        query.get_results::<Note>(c)
    })
    .await
    .unwrap_or(vec![])
}
