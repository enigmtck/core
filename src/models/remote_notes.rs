use std::collections::HashMap;

use crate::activity_pub::{ApAddress, ApNote, ApNoteType};
use crate::db::Db;
use crate::models::to_serde;
use crate::schema::remote_notes;
use crate::{MaybeMultiple, POOL};
use diesel::prelude::*;
use maplit::{hashmap, hashset};

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        use crate::models::pg::notes::NoteType;
        pub fn to_kind(kind: ApNoteType) -> NoteType {
            kind.into()
        }

        pub use crate::models::pg::remote_notes::NewRemoteNote;
        pub use crate::models::pg::remote_notes::RemoteNote;
        pub use crate::models::pg::remote_notes::create_or_update_remote_note;
    } else if #[cfg(feature = "sqlite")] {
        pub fn to_kind(kind: ApNoteType) -> String {
            kind.to_string().to_lowercase()
        }

        pub use crate::models::sqlite::remote_notes::NewRemoteNote;
        pub use crate::models::sqlite::remote_notes::RemoteNote;
        pub use crate::models::sqlite::remote_notes::create_or_update_remote_note;
    }
}

impl From<ApNote> for NewRemoteNote {
    fn from(note: ApNote) -> NewRemoteNote {
        log::debug!("BUILDING NewRemoteNote FROM ApNote\n{note:#?}");

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

        log::debug!("ENTERING DEFINITION");

        NewRemoteNote {
            url: note.clone().url,
            published,
            kind: to_kind(note.clone().kind),
            ap_id: note.clone().id.unwrap(),
            attributed_to: Some(note.attributed_to.to_string()),
            ap_to: to_serde(note.to),
            cc: to_serde(note.cc),
            replies: to_serde(note.replies),
            tag: to_serde(note.tag),
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
            content_map: to_serde(clean_content_map),
            attachment: to_serde(note.attachment),
            ..Default::default()
        }
    }
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
