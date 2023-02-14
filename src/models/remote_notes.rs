use std::collections::{hash_map, HashMap};

use crate::activity_pub::{ApFlexible, ApNote};
use crate::db::Db;
use crate::schema::remote_notes;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use maplit::{hashmap, hashset};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, AsChangeset)]
#[table_name = "remote_notes"]
pub struct NewRemoteNote {
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

impl From<ApNote> for NewRemoteNote {
    fn from(note: ApNote) -> NewRemoteNote {
        let mut ammonia = ammonia::Builder::default();

        ammonia
            .add_tag_attributes("span", &["class"])
            .add_tag_attributes("a", &["class"])
            .tag_attribute_values(hashmap![
                "span" => hashmap![
                    "class" => hashset!["h-card"],
                ],
                "a" => hashmap![
                    "class" => hashset!["u-url mention"],
                ],
            ]);

        let published = match note.clone().published {
            Some(x) => Option::from(x),
            _ => Option::None,
        };

        let clean_content_map = {
            let mut content_map = HashMap::<String, String>::new();
            if let Some(map) = (note).clone().content_map {
                for (key, value) in map {
                    content_map.insert(key, ammonia.clean(&value).to_string());
                }
            }

            content_map
        };

        NewRemoteNote {
            url: note.clone().url,
            published,
            kind: note.clone().kind.to_string(),
            ap_id: note.clone().id.unwrap(),
            attributed_to: Some(note.attributed_to),
            ap_to: Option::from(serde_json::to_value(&note.to).unwrap()),
            cc: Option::from(serde_json::to_value(&note.cc).unwrap()),
            replies: Option::from(serde_json::to_value(&note.replies).unwrap()),
            tag: Option::from(serde_json::to_value(&note.tag).unwrap()),
            content: ammonia.clean(&note.content).to_string(),
            summary: {
                if let Some(summary) = note.summary {
                    Option::from(ammonia.clean(&summary).to_string())
                } else {
                    Option::None
                }
            },
            ap_sensitive: note.sensitive,
            atom_uri: note.atom_uri,
            in_reply_to: note.in_reply_to,
            in_reply_to_atom_uri: note.in_reply_to_atom_uri,
            conversation: note.conversation,
            content_map: Option::from(serde_json::to_value(clean_content_map).unwrap()),
            attachment: Option::from(serde_json::to_value(&note.attachment).unwrap()),
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

pub async fn delete_remote_note_by_ap_id(conn: &Db, ap_id: String) -> Result<(), ()> {
    use crate::schema::remote_notes::dsl::{ap_id as a, remote_notes};

    match conn
        .run(move |c| diesel::delete(remote_notes.filter(a.eq(ap_id))).execute(c))
        .await
    {
        Ok(_) => Ok(()),
        Err(_) => Err(()),
    }
}

pub async fn create_or_update_remote_note(conn: &Db, note: NewRemoteNote) -> Option<RemoteNote> {
    match conn
        .run(move |c| {
            diesel::insert_into(remote_notes::table)
                .values(&note)
                .on_conflict(remote_notes::ap_id)
                .do_update()
                .set(&note)
                .get_result::<RemoteNote>(c)
                .optional()
        })
        .await
    {
        Ok(x) => x,
        Err(e) => {
            log::error!("DATABASE UPDATE FAILURE: {e:#?}");
            Option::None
        }
    }
}
