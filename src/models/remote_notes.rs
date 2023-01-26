use crate::activity_pub::{ApFlexible, ApNote};
use crate::db::Db;
use crate::schema::remote_notes;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
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
    pub summary: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub atom_uri: Option<String>,
    pub in_reply_to: Option<String>,
    pub in_reply_to_atom_uri: Option<String>,
    pub conversation: Option<String>,
    pub content_map: Option<Value>,
}

type IdentifiedApNote = (ApNote, i32);
impl From<IdentifiedApNote> for NewRemoteNote {
    fn from(note: IdentifiedApNote) -> NewRemoteNote {
        let url = match note.clone().0.url {
            Some(ApFlexible::Single(x)) => Option::from(x.as_str().unwrap().to_string()),
            _ => Option::None,
        };

        let published = match note.clone().0.published {
            Some(x) => Option::from(x),
            _ => Option::None,
        };

        NewRemoteNote {
            url,
            published,
            kind: note.0.clone().kind.to_string(),
            ap_id: note.0.clone().id.unwrap(),
            attributed_to: Some(note.0.attributed_to),
            ap_to: Option::from(serde_json::to_value(&note.0.to).unwrap()),
            cc: Option::from(serde_json::to_value(&note.0.cc).unwrap()),
            replies: Option::from(serde_json::to_value(&note.0.replies).unwrap()),
            tag: Option::from(serde_json::to_value(&note.0.tag).unwrap()),
            content: note.0.content,
            profile_id: note.1,
            summary: note.0.summary,
            ap_sensitive: note.0.sensitive,
            atom_uri: note.0.atom_uri,
            in_reply_to: note.0.in_reply_to,
            in_reply_to_atom_uri: note.0.in_reply_to_atom_uri,
            conversation: note.0.conversation,
            content_map: note.0.content_map,
            ..Default::default()
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Default, Debug)]
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
    pub attributed_to: String,
    pub content: String,
    pub attachment: Option<Value>,
    pub replies: Option<Value>,
    pub in_reply_to: Option<String>,
    pub signature: Option<Value>,
    pub summary: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub atom_uri: Option<String>,
    pub in_reply_to_atom_uri: Option<String>,
    pub conversation: Option<String>,
    pub content_map: Option<Value>,
}

pub async fn get_remote_note_by_ap_id(conn: &crate::db::Db, ap_id: String) -> Option<RemoteNote> {
    use crate::schema::remote_notes::dsl::{ap_id as a, remote_notes};

    match conn
        .run(move |c| remote_notes.filter(a.eq(ap_id)).first::<RemoteNote>(c))
        .await
    {
        Ok(x) => Option::from(x),
        Err(_) => Option::None,
    }
}
