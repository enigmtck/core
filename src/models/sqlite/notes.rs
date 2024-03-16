use crate::activity_pub::{ApNote, ApNoteType};
use crate::db::Db;
use crate::helper::{
    get_local_identifier, get_note_ap_id_from_uuid, handle_option, is_local, LocalIdentifierType,
};
use crate::schema::notes;
use crate::POOL;
use anyhow::Result;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::remote_notes::{get_remote_note_by_ap_id, RemoteNote};

#[derive(Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
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

pub type IdentifiedApNote = (ApNote, i32);

impl From<IdentifiedApNote> for NewNote {
    fn from((note, profile_id): IdentifiedApNote) -> Self {
        let uuid = uuid::Uuid::new_v4().to_string();

        NewNote {
            profile_id,
            uuid: uuid.clone(),
            kind: note.clone().kind.to_string().to_lowercase(),
            ap_to: serde_json::to_string(&note.to).unwrap(),
            attributed_to: note.attributed_to.to_string(),
            tag: handle_option(serde_json::to_string(&note.tag).unwrap()),
            attachment: handle_option(serde_json::to_string(&note.attachment).unwrap()),
            instrument: handle_option(serde_json::to_string(&note.instrument).unwrap()),
            content: note.content,
            in_reply_to: note.in_reply_to,
            cc: handle_option(serde_json::to_string(&note.cc).unwrap()),
            conversation: {
                if note.conversation.is_none() {
                    Option::from(format!(
                        "{}/conversation/{}",
                        *crate::SERVER_URL,
                        uuid::Uuid::new_v4()
                    ))
                } else {
                    note.conversation
                }
            },
            // I think this fn will only be used when submitting a new ApNote via an outbox call by
            // the client - in that case we'd never want to accept an id from the client; this logic
            // would better be to just always use the UUID as the basis for the ap_id
            ap_id: note.id.map_or(Some(get_note_ap_id_from_uuid(uuid)), Some),
        }
    }
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

pub async fn get_note_by_uuid(conn: Option<&Db>, uuid: String) -> Option<Note> {
    match conn {
        Some(conn) => conn
            .run(move |c| notes::table.filter(notes::uuid.eq(uuid)).first::<Note>(c))
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            notes::table
                .filter(notes::uuid.eq(uuid))
                .first::<Note>(&mut pool)
                .ok()
        }
    }
}

pub async fn get_note_by_apid(conn: &Db, ap_id: String) -> Option<Note> {
    conn.run(move |c| notes::table.filter(notes::ap_id.eq(ap_id)).first::<Note>(c))
        .await
        .ok()
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

#[derive(Debug)]
pub enum DeleteNoteError {
    ConnectionError,
    DatabaseError(diesel::result::Error),
}

pub async fn delete_note_by_uuid(conn: Option<&Db>, uuid: String) -> Result<usize> {
    match conn {
        Some(conn) => conn
            .run(move |c| diesel::delete(notes::table.filter(notes::uuid.eq(uuid))).execute(c))
            .await
            .map_err(anyhow::Error::msg),
        None => {
            let mut pool = POOL.get()?;
            diesel::delete(notes::table.filter(notes::uuid.eq(uuid)))
                .execute(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}

#[derive(Clone)]
pub enum NoteLike {
    Note(Note),
    RemoteNote(RemoteNote),
}

pub async fn get_notey(conn: &Db, id: String) -> Option<NoteLike> {
    if is_local(id.clone()) {
        let identifier = get_local_identifier(id.clone())?;
        if identifier.kind == LocalIdentifierType::Note {
            let note = get_note_by_uuid(Some(conn), identifier.identifier).await?;
            Some(NoteLike::Note(note))
        } else {
            None
        }
    } else {
        let remote_note = get_remote_note_by_ap_id(Some(conn), id).await?;
        Some(NoteLike::RemoteNote(remote_note))
    }
}
