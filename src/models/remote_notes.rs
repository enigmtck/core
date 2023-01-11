use crate::activity_pub::{ApFlexible, ApNote};
use crate::schema::remote_notes;
use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[table_name = "remote_notes"]
pub struct NewRemoteNote {
    pub profile_id: i32,
    pub kind: String,
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
}

impl From<ApNote> for NewRemoteNote {
    fn from(note: ApNote) -> NewRemoteNote {
        let url = match note.url.clone() {
            Some(ApFlexible::Single(x)) => Option::from(x.as_str().unwrap().to_string()),
            _ => Option::None,
        };

        let published = match note.published.clone() {
            Some(x) => Option::from(x),
            _ => Option::None,
        };

        NewRemoteNote {
            url,
            published,
            kind: note.kind.to_string(),
            ap_id: note.id.unwrap(),
            attributed_to: Some(note.attributed_to),
            ap_to: Option::from(serde_json::to_value(&note.to).unwrap()),
            cc: Option::from(serde_json::to_value(&note.cc).unwrap()),
            replies: Option::from(serde_json::to_value(&note.replies).unwrap()),
            tag: Option::from(serde_json::to_value(&note.tag).unwrap()),
            content: note.content,
            ..Default::default()
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "remote_notes"]
pub struct RemoteNote {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile_id: i32,
    pub kind: String,
    pub ap_id: String,
    pub published: Option<String>,
    pub url: Option<String>,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub tag: Option<Value>,
    pub attributed_to: Option<String>,
    pub content: String,
    pub attachment: Option<Value>,
    pub replies: Option<Value>,
    pub in_reply_to: Option<String>,
    pub signature: Option<Value>,
}
