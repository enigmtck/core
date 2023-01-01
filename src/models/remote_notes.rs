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
        if let Some(ApFlexible::Single(attributed_to)) = note.base.attributed_to {
            let url = match note.base.url.clone() {
                Some(ApFlexible::Single(x)) => Option::from(x.as_str().unwrap().to_string()),
                _ => Option::None,
            };

            let published = match note.base.published.clone() {
                Some(x) => Option::from(x),
                _ => Option::None,
            };

            NewRemoteNote {
                url,
                published,
                ap_id: note.base.id.unwrap(),
                attributed_to: Some(attributed_to.as_str().unwrap().to_string()),
                ap_to: Option::from(serde_json::to_value(&note.to).unwrap()),
                cc: Option::from(serde_json::to_value(&note.base.cc).unwrap()),
                replies: Option::from(serde_json::to_value(&note.base.replies).unwrap()),
                tag: Option::from(serde_json::to_value(&note.base.tag).unwrap()),
                content: note.content,
                ..Default::default()
            }
        } else {
            NewRemoteNote {
                ..Default::default()
            }
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
