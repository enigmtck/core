use std::collections::HashMap;

use crate::activity_pub::{ApAddress, ApNote, ApNoteType, ApObject, ApQuestion, ApQuestionType};
use crate::db::Db;
use crate::models::{to_serde, to_time};
use crate::schema::objects;
use crate::{MaybeMultiple, POOL};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use maplit::{hashmap, hashset};

use super::actors::Actor;
use super::from_serde;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        use crate::models::pg::objects::ObjectType;
        // pub fn to_kind(kind: ApNoteType) -> NoteType {
        //     kind.into()
        // }

        pub use crate::models::pg::objects::NewObject;
        pub use crate::models::pg::objects::Object;
        pub use crate::models::pg::objects::create_or_update_object;
    } else if #[cfg(feature = "sqlite")] {
        // pub fn to_kind(kind: ApNoteType) -> String {
        //     kind.to_string().to_lowercase()
        // }

        // pub use crate::models::sqlite::remote_notes::NewRemoteNote;
        // pub use crate::models::sqlite::remote_notes::RemoteNote;
        // pub use crate::models::sqlite::remote_notes::create_or_update_remote_note;
    }
}

impl From<ApNoteType> for ObjectType {
    fn from(kind: ApNoteType) -> Self {
        match kind {
            ApNoteType::EncryptedNote => ObjectType::Note,
            ApNoteType::Note => ObjectType::Note,
            ApNoteType::VaultNote => ObjectType::Note,
        }
    }
}

impl From<ApQuestionType> for ObjectType {
    fn from(kind: ApQuestionType) -> Self {
        match kind {
            ApQuestionType::Question => ObjectType::Question,
        }
    }
}

impl TryFrom<ApObject> for NewObject {
    type Error = anyhow::Error;

    fn try_from(object: ApObject) -> Result<Self, Self::Error> {
        match object {
            ApObject::Note(note) => Ok(note.into()),
            ApObject::Question(question) => Ok(question.into()),
            _ => Err(anyhow!(
                "conversion to NewObject not implemented: {object:#?}"
            )),
        }
    }
}

type AttributedApNote = (ApNote, Actor);

impl From<AttributedApNote> for NewObject {
    fn from((note, actor): AttributedApNote) -> NewObject {
        let mut object: NewObject = note.into();
        object.ek_profile_id = Some(actor.id);

        object.clone()
    }
}

impl From<ApNote> for NewObject {
    fn from(note: ApNote) -> NewObject {
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

        let published: Option<DateTime<Utc>> = note
            .clone()
            .published
            .parse::<DateTime<chrono::FixedOffset>>()
            .ok()
            .map(|dt| dt.with_timezone(&Utc));

        let clean_content_map = {
            let mut content_map = HashMap::<String, String>::new();
            if let Some(map) = (note).clone().content_map {
                for (key, value) in map {
                    content_map.insert(key, ammonia.clean(&value).to_string());
                }
            }

            content_map
        };

        NewObject {
            as_url: serde_json::to_value(note.clone().url).ok(),
            as_published: published,
            as_type: note.clone().kind.into(),
            as_id: note.id.clone().unwrap(),
            as_attributed_to: to_serde(&Some(note.attributed_to.to_string())),
            as_to: to_serde(&Some(note.to)),
            as_cc: to_serde(&note.cc),
            as_replies: to_serde(&note.replies),
            as_tag: to_serde(&note.tag),
            as_content: Some(ammonia.clean(&note.content).to_string()),
            as_summary: note.summary.map(|x| ammonia.clean(&x).to_string()),
            ap_sensitive: note.sensitive,
            as_in_reply_to: to_serde(&note.in_reply_to),
            ap_conversation: note.conversation,
            as_content_map: to_serde(&Some(clean_content_map)),
            as_attachment: to_serde(&note.attachment),
            ek_uuid: note.ephemeral.and_then(|x| x.internal_uuid),
            ..Default::default()
        }
    }
}

impl From<ApQuestion> for NewObject {
    fn from(question: ApQuestion) -> Self {
        NewObject {
            as_type: question.kind.into(),
            as_id: question.id,
            as_to: to_serde(&Some(question.to)),
            as_cc: to_serde(&question.cc),
            as_end_time: question.end_time.map(to_time),
            as_published: question.published.map(to_time),
            as_one_of: to_serde(&question.one_of),
            as_any_of: to_serde(&question.any_of),
            as_content: question.content,
            as_content_map: to_serde(&question.content_map),
            as_summary: question.summary,
            ap_voters_count: question.voters_count,
            as_url: to_serde(&question.url),
            ap_conversation: question.conversation,
            as_tag: to_serde(&question.tag),
            as_attachment: to_serde(&question.attachment),
            ap_sensitive: question.sensitive,
            as_in_reply_to: to_serde(&question.in_reply_to),
            as_attributed_to: to_serde(&Some(question.attributed_to.to_string())),
            ..Default::default()
        }
    }
}

impl Object {
    pub fn is_public(&self) -> bool {
        if let Some(to) = from_serde::<MaybeMultiple<ApAddress>>(self.as_to.clone().into()) {
            for address in to.multiple() {
                if address.is_public() {
                    return true;
                }
            }
        }

        if let Some(cc) = from_serde::<MaybeMultiple<ApAddress>>(self.as_cc.clone().into()) {
            for address in cc.multiple() {
                if address.is_public() {
                    return true;
                }
            }
        }

        false
    }
}

pub async fn get_object(conn: Option<&Db>, id: i32) -> Result<Object> {
    match conn {
        Some(conn) => conn
            .run(move |c| objects::table.find(id).first::<Object>(c))
            .await
            .map_err(anyhow::Error::msg),
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            objects::table
                .find(id)
                .first::<Object>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}

pub async fn get_object_by_as_id(conn: Option<&Db>, as_id: String) -> Result<Object> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                objects::table
                    .filter(objects::as_id.eq(as_id))
                    .first::<Object>(c)
            })
            .await
            .map_err(anyhow::Error::msg),
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            objects::table
                .filter(objects::as_id.eq(as_id))
                .first::<Object>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}

pub async fn get_object_by_uuid(conn: Option<&Db>, uuid: String) -> Result<Object> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                objects::table
                    .filter(objects::ek_uuid.eq(uuid))
                    .first::<Object>(c)
            })
            .await
            .map_err(anyhow::Error::msg),
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            objects::table
                .filter(objects::ek_uuid.eq(uuid))
                .first::<Object>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}

pub async fn tombstone_object_by_as_id(conn: &Db, as_id: String) -> Result<Object> {
    conn.run(move |c| {
        diesel::update(objects::table.filter(objects::as_id.eq(as_id)))
            .set(objects::as_type.eq(ObjectType::Tombstone))
            .get_result(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn tombstone_object_by_uuid(conn: &Db, uuid: String) -> Result<Object> {
    conn.run(move |c| {
        diesel::update(objects::table.filter(objects::ek_uuid.eq(uuid)))
            .set(objects::as_type.eq(ObjectType::Tombstone))
            .get_result(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn delete_object_by_as_id(conn: &Db, as_id: String) -> Result<usize> {
    conn.run(move |c| diesel::delete(objects::table.filter(objects::as_id.eq(as_id))).execute(c))
        .await
        .map_err(anyhow::Error::msg)
}

pub async fn delete_object_by_uuid(conn: &Db, uuid: String) -> Result<usize> {
    conn.run(move |c| diesel::delete(objects::table.filter(objects::ek_uuid.eq(uuid))).execute(c))
        .await
        .map_err(anyhow::Error::msg)
}
