use std::collections::HashMap;

use crate::activity_pub::{ApAddress, ApNote};
use crate::db::Db;
use crate::schema::remote_notes;
use crate::{MaybeMultiple, POOL};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use maplit::{hashmap, hashset};
use serde::{Deserialize, Serialize};

use super::notes::NoteType;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, AsChangeset)]
#[diesel(table_name = remote_notes)]
pub struct NewRemoteNote {
    pub kind: String,
    pub ap_id: String,
    pub published: Option<String>,
    pub url: Option<String>,
    pub attributed_to: Option<String>,
    pub ap_to: Option<String>,
    pub cc: Option<String>,
    pub content: String,
    pub attachment: Option<String>,
    pub tag: Option<String>,
    pub replies: Option<String>,
    pub signature: Option<String>,
    pub summary: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub atom_uri: Option<String>,
    pub in_reply_to: Option<String>,
    pub in_reply_to_atom_uri: Option<String>,
    pub conversation: Option<String>,
    pub content_map: Option<String>,
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

        let published = Some(note.clone().published);

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
            kind: String::from(NoteType::from(note.clone().kind)),
            ap_id: note.clone().id.unwrap(),
            attributed_to: Some(note.attributed_to.to_string()),
            ap_to: Option::from(serde_json::to_string(&note.to).unwrap()),
            cc: Option::from(serde_json::to_string(&note.cc).unwrap()),
            replies: Option::from(serde_json::to_string(&note.replies).unwrap()),
            tag: Option::from(serde_json::to_string(&note.tag).unwrap()),
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
            content_map: Option::from(serde_json::to_string(&clean_content_map).unwrap()),
            attachment: Option::from(serde_json::to_string(&note.attachment).unwrap()),
            ..Default::default()
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Default, Debug)]
#[diesel(table_name = remote_notes)]
pub struct RemoteNote {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub kind: String,
    pub ap_id: String,
    pub published: Option<String>,
    pub url: Option<String>,
    pub ap_to: Option<String>,
    pub cc: Option<String>,
    pub tag: Option<String>,
    pub attributed_to: String,
    pub content: String,
    pub attachment: Option<String>,
    pub replies: Option<String>,
    pub in_reply_to: Option<String>,
    pub signature: Option<String>,
    pub summary: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub atom_uri: Option<String>,
    pub in_reply_to_atom_uri: Option<String>,
    pub conversation: Option<String>,
    pub content_map: Option<String>,
}

impl RemoteNote {
    pub fn is_public(&self) -> bool {
        if let Ok(to) =
            serde_json::from_value::<MaybeMultiple<ApAddress>>(self.ap_to.clone().into())
        {
            for address in to.multiple() {
                if address.is_public() {
                    return true;
                }
            }
        }

        if let Ok(cc) = serde_json::from_value::<MaybeMultiple<ApAddress>>(self.cc.clone().into()) {
            for address in cc.multiple() {
                if address.is_public() {
                    return true;
                }
            }
        }

        false
    }
}

pub async fn get_remote_note_by_ap_id(conn: Option<&Db>, ap_id: String) -> Option<RemoteNote> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                remote_notes::table
                    .filter(remote_notes::ap_id.eq(ap_id))
                    .first::<RemoteNote>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            remote_notes::table
                .filter(remote_notes::ap_id.eq(ap_id))
                .first::<RemoteNote>(&mut pool)
                .ok()
        }
    }
}

pub async fn delete_remote_note_by_ap_id(conn: &Db, ap_id: String) -> bool {
    conn.run(move |c| {
        diesel::delete(remote_notes::table.filter(remote_notes::ap_id.eq(ap_id))).execute(c)
    })
    .await
    .is_ok()
}

pub async fn create_or_update_remote_note(
    conn: Option<&Db>,
    note: NewRemoteNote,
) -> Option<RemoteNote> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(remote_notes::table)
                    .values(&note)
                    .on_conflict(remote_notes::ap_id)
                    .do_update()
                    .set(&note)
                    .execute(c)
                    .ok()?;

                remote_notes::table
                    .filter(remote_notes::ap_id.eq(&note.ap_id))
                    .first::<RemoteNote>(c)
                    .ok()
            })
            .await
        }
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(remote_notes::table)
                .values(&note)
                .on_conflict(remote_notes::ap_id)
                .do_update()
                .set(&note)
                .execute(&mut pool)
                .ok()?;

            remote_notes::table
                .filter(remote_notes::ap_id.eq(&note.ap_id))
                .first::<RemoteNote>(&mut pool)
                .ok()
        }
    }
}
