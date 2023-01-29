use crate::activity_pub::ApNote;
use crate::db::Db;
use crate::schema::notes;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[table_name = "notes"]
pub struct NewNote {
    pub uuid: String,
    pub profile_id: i32,
    pub content: String,
    pub kind: String,
    pub ap_to: Value,
    pub attributed_to: String,
    pub in_reply_to: Option<String>,
    pub tag: Option<Value>,
    pub cc: Option<Value>,
}

pub type IdentifiedApNote = (ApNote, i32);

impl From<IdentifiedApNote> for NewNote {
    fn from(note: IdentifiedApNote) -> Self {
        NewNote {
            profile_id: note.1,
            uuid: uuid::Uuid::new_v4().to_string(),
            kind: note.0.kind.to_string(),
            ap_to: serde_json::to_value(&note.0.to).unwrap(),
            attributed_to: note.0.attributed_to,
            tag: Option::from(serde_json::to_value(&note.0.tag).unwrap()),
            content: note.0.content,
            in_reply_to: note.0.in_reply_to,
            cc: Option::None,
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "notes"]
pub struct Note {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub profile_id: i32,
    pub kind: String,
    pub ap_to: Value,
    pub cc: Option<Value>,
    pub tag: Option<Value>,
    pub attributed_to: String,
    pub in_reply_to: Option<String>,
    pub content: String,
}

pub async fn get_note_by_uuid(conn: &Db, uuid: String) -> Option<Note> {
    match conn
        .run(move |c| notes::table.filter(notes::uuid.eq(uuid)).first::<Note>(c))
        .await
    {
        Ok(x) => Option::from(x),
        Err(_) => Option::None,
    }
}
