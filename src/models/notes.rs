use crate::activity_pub::ApNote;
use crate::db::Db;
use crate::helper::handle_option;
use crate::schema::notes;
use crate::MaybeMultiple;
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
    pub attachment: Option<Value>,
    pub cc: Option<Value>,
    pub conversation: Option<String>,
    pub instrument: Option<Value>,
}

pub type IdentifiedApNote = (ApNote, i32);

impl From<IdentifiedApNote> for NewNote {
    fn from((note, profile_id): IdentifiedApNote) -> Self {
        NewNote {
            profile_id,
            uuid: uuid::Uuid::new_v4().to_string(),
            kind: note.kind.to_string(),
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
    pub conversation: Option<String>,
    pub attachment: Option<Value>,
    pub instrument: Option<Value>,
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

impl Note {
    // TODO: This should probably be handled by ApAddress
    pub fn is_public(&self) -> bool {
        if let Ok(to) = serde_json::from_value::<MaybeMultiple<String>>(self.ap_to.clone()) {
            match to {
                MaybeMultiple::Multiple(n) => {
                    n.contains(&"https://www.w3.org/ns/activitystreams#Public".to_string())
                }
                MaybeMultiple::Single(n) => n == *"https://www.w3.org/ns/activitystreams#Public",
                MaybeMultiple::None => false,
            }
        } else {
            false
        }
    }
}
