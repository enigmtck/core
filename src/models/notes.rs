use crate::activity_pub::{ApNote, ApNoteType};
use crate::db::Db;
use crate::helper::{
    get_local_identifier, get_note_ap_id_from_uuid, is_local, LocalIdentifierType,
};
use crate::schema::notes;
use crate::POOL;
use anyhow::Result;
use diesel::prelude::*;

use super::objects::{get_object_by_as_id, Object};
use super::to_serde;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub fn to_kind(kind: ApNoteType) -> NoteType {
            kind.into()
        }

        pub use super::pg::notes::NoteType;
        pub use super::pg::notes::NewNote;
        pub use super::pg::notes::Note;
        pub use super::pg::notes::create_note;
        pub use super::pg::notes::get_notes_by_profile_id;
    } else if #[cfg(feature = "sqlite")] {
        pub fn to_kind(kind: ApNoteType) -> String {
            kind.to_string().to_lowercase()
        }

        pub use super::sqlite::notes::NoteType;
        pub use super::sqlite::notes::NewNote;
        pub use super::sqlite::notes::Note;
        pub use super::sqlite::notes::create_note;
        pub use super::sqlite::notes::get_notes_by_profile_id;
    }
}

pub type IdentifiedApNote = (ApNote, i32);

impl From<IdentifiedApNote> for NewNote {
    fn from((note, profile_id): IdentifiedApNote) -> Self {
        let uuid = uuid::Uuid::new_v4().to_string();

        NewNote {
            profile_id,
            uuid: uuid.clone(),
            kind: to_kind(note.clone().kind),
            ap_to: to_serde(note.to).unwrap(),
            attributed_to: note.attributed_to.to_string(),
            tag: to_serde(note.tag),
            attachment: to_serde(note.attachment),
            instrument: to_serde(note.instrument),
            content: note.content,
            in_reply_to: note.in_reply_to,
            cc: to_serde(note.cc),
            conversation: {
                if note.conversation.is_none() {
                    Some(format!(
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
    Object(Object),
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
        let object = get_object_by_as_id(Some(conn), id).await.ok()?;
        Some(NoteLike::Object(object))
    }
}
