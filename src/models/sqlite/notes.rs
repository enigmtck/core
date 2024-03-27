use crate::db::Db;
use crate::schema::notes;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NoteType {
    #[default]
    Note,
    EncryptedNote,
    VaultNote,
}

impl fmt::Display for NoteType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = notes)]
pub struct NewNote {
    pub uuid: String,
    pub profile_id: i32,
    pub content: String,
    pub kind: String,
    pub ap_to: String,
    pub attributed_to: String,
    pub in_reply_to: Option<String>,
    pub tag: Option<String>,
    pub attachment: Option<String>,
    pub cc: Option<String>,
    pub conversation: Option<String>,
    pub instrument: Option<String>,
    pub ap_id: Option<String>,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = notes)]
pub struct Note {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub uuid: String,
    pub profile_id: i32,
    pub kind: String,
    pub ap_to: String,
    pub cc: Option<String>,
    pub tag: Option<String>,
    pub attributed_to: String,
    pub in_reply_to: Option<String>,
    pub content: String,
    pub conversation: Option<String>,
    pub attachment: Option<String>,
    pub instrument: Option<String>,
    pub ap_id: Option<String>,
}

pub async fn create_note(conn: &Db, note: NewNote) -> Option<Note> {
    // Execute the insert operation.
    conn.run(move |c| diesel::insert_into(notes::table).values(&note).execute(c))
        .await
        .ok()?;

    conn.run(move |c| notes::table.order(notes::id.desc()).first::<Note>(c))
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
            .filter(notes::kind.eq("note".to_string()))
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
