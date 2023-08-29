use crate::activity_pub::{ApNote, ApNoteType};
use crate::db::Db;
use crate::helper::{get_note_ap_id_from_uuid, handle_option};
use crate::schema::notes;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(
    diesel_derive_enum::DbEnum, Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq,
)]
#[ExistingTypePath = "crate::schema::sql_types::NoteType"]
pub enum NoteType {
    #[default]
    Note,
    EncryptedNote,
    VaultNote,
}

impl From<ApNoteType> for NoteType {
    fn from(kind: ApNoteType) -> Self {
        match kind {
            ApNoteType::EncryptedNote => NoteType::EncryptedNote,
            ApNoteType::Note => NoteType::Note,
            ApNoteType::VaultNote => NoteType::VaultNote,
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

pub type IdentifiedApNote = (ApNote, i32);

impl From<IdentifiedApNote> for NewNote {
    fn from((note, profile_id): IdentifiedApNote) -> Self {
        let uuid = uuid::Uuid::new_v4().to_string();

        NewNote {
            profile_id,
            uuid: uuid.clone(),
            kind: note.kind.into(),
            ap_to: serde_json::to_value(&note.to).unwrap(),
            attributed_to: note.attributed_to.to_string(),
            tag: handle_option(serde_json::to_value(&note.tag).unwrap()),
            attachment: handle_option(serde_json::to_value(&note.attachment).unwrap()),
            instrument: handle_option(serde_json::to_value(&note.instrument).unwrap()),
            content: note.content,
            in_reply_to: note.in_reply_to,
            cc: handle_option(serde_json::to_value(&note.cc).unwrap()),
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

pub async fn get_note_by_uuid(conn: &Db, uuid: String) -> Option<Note> {
    conn.run(move |c| notes::table.filter(notes::uuid.eq(uuid)).first::<Note>(c))
        .await
        .ok()
}

pub async fn get_note_by_apid(conn: &Db, ap_id: String) -> Option<Note> {
    conn.run(move |c| notes::table.filter(notes::ap_id.eq(ap_id)).first::<Note>(c))
        .await
        .ok()
}
