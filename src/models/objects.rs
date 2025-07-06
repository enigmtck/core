use super::actors::Actor;
use super::{coalesced_activity::CoalescedActivity, from_serde};
use crate::db::runner::DbRunner;
use crate::schema::objects;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use convert_case::{Case, Casing};
use diesel::prelude::*;
use diesel::{sql_query, AsChangeset, Identifiable, Insertable, Queryable};
use jdt_activity_pub::MaybeMultiple;
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{
    ApAddress, ApArticle, ApDateTime, ApHashtag, ApNote, ApNoteType, ApObject, ApQuestion,
    ApQuestionType, Ephemeral,
};
use maplit::{hashmap, hashset};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::{self, Debug};

#[derive(
    diesel_derive_enum::DbEnum, Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq,
)]
#[ExistingTypePath = "crate::schema::sql_types::ObjectType"]
pub enum ObjectType {
    Article,
    Audio,
    Document,
    Event,
    Image,
    #[default]
    Note,
    Page,
    Place,
    Profile,
    Question,
    Relationship,
    Tombstone,
    Video,
    EncryptedNote,
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl ObjectType {
    pub fn is_note(&self) -> bool {
        matches!(self, ObjectType::Note)
    }

    pub fn is_encrypted_note(&self) -> bool {
        matches!(self, ObjectType::EncryptedNote)
    }

    pub fn is_question(&self) -> bool {
        matches!(self, ObjectType::Question)
    }

    pub fn is_article(&self) -> bool {
        matches!(self, ObjectType::Article)
    }
}

impl TryFrom<String> for ObjectType {
    type Error = anyhow::Error;

    fn try_from(object: String) -> Result<Self, Self::Error> {
        match object.to_case(Case::Snake).as_str() {
            "article" => Ok(ObjectType::Article),
            "audio" => Ok(ObjectType::Audio),
            "document" => Ok(ObjectType::Document),
            "event" => Ok(ObjectType::Event),
            "image" => Ok(ObjectType::Image),
            "note" => Ok(ObjectType::Note),
            "page" => Ok(ObjectType::Page),
            "place" => Ok(ObjectType::Place),
            "profile" => Ok(ObjectType::Profile),
            "question" => Ok(ObjectType::Question),
            "relationship" => Ok(ObjectType::Relationship),
            "tombstone" => Ok(ObjectType::Tombstone),
            "video" => Ok(ObjectType::Video),
            "encrypted_note" => Ok(ObjectType::EncryptedNote),
            _ => Err(anyhow!("unimplemented ObjectType")),
        }
    }
}

impl From<ObjectType> for String {
    fn from(object: ObjectType) -> Self {
        format!("{object}").to_case(Case::Snake)
    }
}

impl TryFrom<ObjectType> for ApNoteType {
    type Error = anyhow::Error;

    fn try_from(kind: ObjectType) -> Result<Self, Self::Error> {
        match kind {
            ObjectType::Note => Ok(Self::Note),
            ObjectType::EncryptedNote => Ok(Self::EncryptedNote),
            _ => Err(anyhow!("invalid Object type for ApNote")),
        }
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, AsChangeset, Clone)]
#[diesel(table_name = objects)]
pub struct NewObject {
    pub ap_conversation: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub ap_signature: Option<Value>,
    pub ap_voters_count: Option<i32>,
    pub as_any_of: Option<Value>,
    pub as_attachment: Option<Value>,
    pub as_attributed_to: Option<Value>,
    pub as_audience: Option<Value>,
    pub as_bcc: Option<Value>,
    pub as_bto: Option<Value>,
    pub as_cc: Option<Value>,
    pub as_closed: Option<Value>,
    pub as_content: Option<String>,
    pub as_content_map: Option<Value>,
    pub as_context: Option<Value>,
    pub as_deleted: Option<DateTime<Utc>>,
    pub as_describes: Option<Value>,
    pub as_duration: Option<String>,
    pub as_end_time: Option<DateTime<Utc>>,
    pub as_former_type: Option<String>,
    pub as_generator: Option<Value>,
    pub as_icon: Option<Value>,
    pub as_id: String,
    pub as_image: Option<Value>,
    pub as_location: Option<Value>,
    pub as_media_type: Option<String>,
    pub as_name: Option<String>,
    pub as_name_map: Option<Value>,
    pub as_one_of: Option<Value>,
    pub as_preview: Option<Value>,
    pub as_published: Option<DateTime<Utc>>,
    pub as_replies: Option<Value>,
    pub as_start_time: Option<DateTime<Utc>>,
    pub as_summary: Option<String>,
    pub as_summary_map: Option<Value>,
    pub as_tag: Option<Value>,
    pub as_to: Option<Value>,

    #[cfg(feature = "pg")]
    pub as_type: ObjectType,

    #[cfg(feature = "sqlite")]
    pub as_type: String,

    pub as_updated: Option<DateTime<Utc>>,
    pub as_url: Option<Value>,
    pub ek_hashtags: Value,
    pub ek_instrument: Option<Value>,
    pub ek_metadata: Option<Value>,
    pub ek_profile_id: Option<i32>,
    pub ek_uuid: Option<String>,
    pub as_in_reply_to: Option<Value>,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Default, Debug)]
#[diesel(table_name = objects)]
pub struct Object {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ap_conversation: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub ap_signature: Option<Value>,
    pub ap_voters_count: Option<i32>,
    pub as_any_of: Option<Value>,
    pub as_attachment: Option<Value>,
    pub as_attributed_to: Option<Value>,
    pub as_audience: Option<Value>,
    pub as_bcc: Option<Value>,
    pub as_bto: Option<Value>,
    pub as_cc: Option<Value>,
    pub as_closed: Option<Value>,
    pub as_content: Option<String>,
    pub as_content_map: Option<Value>,
    pub as_context: Option<Value>,
    pub as_deleted: Option<DateTime<Utc>>,
    pub as_describes: Option<Value>,
    pub as_duration: Option<String>,
    pub as_end_time: Option<DateTime<Utc>>,
    pub as_former_type: Option<String>,
    pub as_generator: Option<Value>,
    pub as_icon: Option<Value>,
    pub as_id: String,
    pub as_image: Option<Value>,
    pub as_location: Option<Value>,
    pub as_media_type: Option<String>,
    pub as_name: Option<String>,
    pub as_name_map: Option<Value>,
    pub as_one_of: Option<Value>,
    pub as_preview: Option<Value>,
    pub as_published: Option<DateTime<Utc>>,
    pub as_replies: Option<Value>,
    pub as_start_time: Option<DateTime<Utc>>,
    pub as_summary: Option<String>,
    pub as_summary_map: Option<Value>,
    pub as_tag: Option<Value>,
    pub as_to: Option<Value>,

    #[cfg(feature = "pg")]
    pub as_type: ObjectType,

    #[cfg(feature = "sqlite")]
    pub as_type: String,

    pub as_updated: Option<DateTime<Utc>>,
    pub as_url: Option<Value>,
    pub ek_hashtags: Value,
    pub ek_instrument: Option<Value>,
    pub ek_metadata: Option<Value>,
    pub ek_profile_id: Option<i32>,
    pub ek_uuid: Option<String>,
    pub as_in_reply_to: Option<Value>,
}

impl Object {
    pub fn attributed_to(&self) -> Vec<String> {
        if let Some(attributed_to) = self.clone().as_attributed_to {
            serde_json::from_value(attributed_to).unwrap_or_default()
        } else {
            vec![]
        }
    }
}

impl TryFrom<CoalescedActivity> for Object {
    type Error = anyhow::Error;

    fn try_from(activity: CoalescedActivity) -> Result<Self, Self::Error> {
        Ok(Object {
            id: activity.target_object_id.unwrap_or(-1),
            created_at: activity
                .object_created_at
                .ok_or(anyhow!("no object created_at"))?,
            updated_at: activity
                .object_updated_at
                .ok_or(anyhow!("no object updated_at"))?,
            ek_uuid: activity.object_uuid,
            as_type: activity.object_type.ok_or(anyhow!("no object type"))?,
            as_published: activity.object_published,
            as_id: activity.object_as_id.ok_or(anyhow!("no object as_id"))?,
            as_url: activity.object_url,
            as_to: activity.object_to,
            as_cc: activity.object_cc,
            as_tag: activity.object_tag,
            as_attributed_to: activity.object_attributed_to,
            as_in_reply_to: activity.object_in_reply_to,
            as_content: activity.object_content,
            ap_conversation: activity.object_conversation,
            as_attachment: activity.object_attachment,
            as_summary: activity.object_summary,
            as_end_time: activity.object_end_time,
            as_one_of: activity.object_one_of,
            as_any_of: activity.object_any_of,
            ap_voters_count: activity.object_voters_count,
            ap_sensitive: activity.object_sensitive,
            ek_metadata: activity.object_metadata,
            ek_profile_id: activity.object_profile_id,
            ..Default::default()
        })
    }
}

impl From<ApNoteType> for ObjectType {
    fn from(kind: ApNoteType) -> Self {
        match kind {
            ApNoteType::EncryptedNote => ObjectType::EncryptedNote,
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
            ApObject::Article(article) => Ok(article.into()),
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

type AttributedApArticle = (ApArticle, Actor);

impl From<AttributedApArticle> for NewObject {
    fn from((article, actor): AttributedApArticle) -> NewObject {
        let mut object: NewObject = article.into();
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

        let published: Option<DateTime<Utc>> = Some(*note.clone().published);

        let clean_content_map = {
            let mut content_map = HashMap::<String, String>::new();
            if let Some(map) = (note).clone().content_map {
                for (key, value) in map {
                    content_map.insert(key, ammonia.clean(&value).to_string());
                }
            }

            content_map
        };

        let hashtags: Vec<ApHashtag> = note.clone().into();
        let ek_hashtags = json!(hashtags
            .iter()
            .map(|x| x.name.clone().to_lowercase())
            .collect::<Vec<String>>());

        NewObject {
            as_url: Some(json!(note.clone().url)),
            as_published: published,
            as_type: note.clone().kind.into(),
            as_id: note.id.clone().expect("note id should not be None"),
            as_attributed_to: Some(json!(note.attributed_to)),
            as_to: note.to.into(),
            as_cc: note.cc.into(),
            as_replies: note.replies.into(),
            as_tag: note.tag.into(),
            as_content: note.content.map(|c| ammonia.clean(&c).to_string()),
            as_summary: note.summary.map(|x| ammonia.clean(&x).to_string()),
            ap_sensitive: note.sensitive,
            as_in_reply_to: note.in_reply_to.into(),
            ap_conversation: note.conversation,
            as_content_map: Some(json!(clean_content_map)),
            as_attachment: note.attachment.into(),
            ek_uuid: note.ephemeral.and_then(|x| x.internal_uuid),
            ek_instrument: note.instrument.option().map(|x| json!(x)),
            ek_hashtags,
            ..Default::default()
        }
    }
}

impl From<ApArticle> for NewObject {
    fn from(article: ApArticle) -> NewObject {
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

        let published: Option<DateTime<Utc>> = Some(*article.clone().published);

        let clean_content_map = {
            let mut content_map = HashMap::<String, String>::new();
            if let Some(map) = article.clone().content_map {
                for (key, value) in map {
                    content_map.insert(key, ammonia.clean(&value).to_string());
                }
            }

            content_map
        };

        let hashtags: Vec<ApHashtag> = article.clone().into();
        let ek_hashtags = json!(hashtags
            .iter()
            .map(|x| x.name.clone().to_lowercase())
            .collect::<Vec<String>>());

        NewObject {
            as_url: Some(json!(article.clone().url)),
            as_published: published,
            as_updated: article.updated.map(|u| *u),
            as_type: ObjectType::Article,
            as_id: article.id.clone().expect("article id should not be None"),
            as_attributed_to: Some(json!(article.attributed_to)),
            as_to: article.to.into(),
            as_cc: article.cc.into(),
            as_bcc: article.bcc.into(),
            as_bto: article.bto.into(),
            as_audience: article.audience.into(),
            as_replies: article.replies.into(),
            as_tag: article.tag.into(),
            as_content: article.content.map(|c| ammonia.clean(&c).to_string()),
            as_summary: article.summary.map(|x| ammonia.clean(&x).to_string()),
            ap_sensitive: article.sensitive,
            as_in_reply_to: article.in_reply_to.into(),
            as_content_map: Some(json!(clean_content_map)),
            as_attachment: article.attachment.into(),
            as_image: article.image.into(),
            as_context: article.context_property.into(),
            as_generator: article.generator.map(|g| json!(g)),
            as_preview: article.preview.into(),
            ek_uuid: article.ephemeral.and_then(|x| x.internal_uuid),
            ek_instrument: article.instrument.option().map(|x| json!(x)),
            ek_hashtags,
            as_name: article.name,
            ..Default::default()
        }
    }
}

impl From<ApQuestion> for NewObject {
    fn from(question: ApQuestion) -> Self {
        NewObject {
            as_type: question.kind.into(),
            as_id: question.id,
            as_to: question.to.into(),
            as_cc: question.cc.into(),
            as_end_time: question.end_time.as_deref().cloned(),
            as_published: question.published.as_deref().cloned(),
            as_one_of: question.one_of.into(),
            as_any_of: question.any_of.into(),
            as_content: question.content,
            as_content_map: question.content_map.map(|x| json!(x)),
            as_summary: question.summary,
            ap_voters_count: question.voters_count,
            as_url: question.url.map(|x| json!(x)),
            ap_conversation: question.conversation,
            as_tag: question.tag.into(),
            as_attachment: question.attachment.into(),
            ap_sensitive: question.sensitive,
            as_in_reply_to: question.in_reply_to.into(),
            as_attributed_to: Some(json!(question.attributed_to.to_string())),
            ..Default::default()
        }
    }
}

impl Object {
    pub fn is_public(&self) -> bool {
        let as_to: MaybeMultiple<ApAddress> = self.as_to.clone().into();

        for address in as_to.multiple() {
            if address.is_public() {
                return true;
            }
        }

        let as_cc: MaybeMultiple<ApAddress> = self.as_cc.clone().into();

        for address in as_cc.multiple() {
            if address.is_public() {
                return true;
            }
        }

        false
    }
}

impl TryFrom<Object> for ApNote {
    type Error = anyhow::Error;

    fn try_from(object: Object) -> Result<ApNote> {
        if object.as_type.is_note() || object.as_type.is_encrypted_note() {
            Ok(ApNote {
                id: Some(object.as_id.clone()),
                kind: object.as_type.try_into()?,
                published: object.as_published.unwrap_or(Utc::now()).into(),
                url: object.as_url.clone().and_then(from_serde),
                to: object
                    .as_to
                    .clone()
                    .and_then(from_serde)
                    .unwrap_or(vec![].into()),
                cc: object.as_cc.clone().into(),
                tag: object.as_tag.clone().into(),
                attributed_to: from_serde(
                    object.as_attributed_to.ok_or(anyhow!("no attributed_to"))?,
                )
                .ok_or(anyhow!("failed to convert from Value"))?,
                content: object.as_content.clone(),
                replies: object
                    .as_replies
                    .clone()
                    .map_or_else(|| MaybeReference::None, |x| x.into()),
                in_reply_to: object.as_in_reply_to.clone().into(),
                attachment: object.as_attachment.clone().into(),
                conversation: object.ap_conversation.clone(),
                ephemeral: Some(Ephemeral {
                    timestamp: Some(object.created_at),
                    metadata: object.ek_metadata.and_then(from_serde),
                    ..Default::default()
                }),
                instrument: object.ek_instrument.clone().into(),
                ..Default::default()
            })
        } else {
            Err(anyhow!("ObjectType is not Note"))
        }
    }
}

impl TryFrom<Object> for ApArticle {
    type Error = anyhow::Error;

    fn try_from(object: Object) -> Result<ApArticle> {
        if object.as_type.is_article() {
            Ok(ApArticle {
                id: Some(object.as_id.clone()),
                kind: jdt_activity_pub::ApArticleType::Article,
                published: object.as_published.unwrap_or(Utc::now()).into(),
                updated: object.as_updated.map(|u| u.into()),
                url: object.as_url.clone().and_then(from_serde),
                to: object
                    .as_to
                    .clone()
                    .and_then(from_serde)
                    .unwrap_or(vec![].into()),
                cc: object.as_cc.clone().into(),
                bcc: object.as_bcc.clone().into(),
                bto: object.as_bto.clone().into(),
                audience: object.as_audience.clone().into(),
                tag: object.as_tag.clone().into(),
                attributed_to: from_serde(
                    object.as_attributed_to.ok_or(anyhow!("no attributed_to"))?,
                )
                .ok_or(anyhow!("failed to convert from Value"))?,
                name: object.as_name.clone(),
                content: object.as_content.clone(),
                summary: object.as_summary.clone(),
                replies: object
                    .as_replies
                    .clone()
                    .map_or_else(|| MaybeReference::None, |x| x.into()),
                in_reply_to: object.as_in_reply_to.clone().into(),
                attachment: object.as_attachment.clone().into(),
                image: object.as_image.clone().into(),
                context_property: object.as_context.clone().into(),
                generator: object.as_generator.clone().and_then(from_serde),
                preview: match object.as_preview {
                    Some(value) => MaybeReference::from(value),
                    None => MaybeReference::None,
                },
                sensitive: object.ap_sensitive,
                content_map: object.as_content_map.clone().and_then(from_serde),
                source: None,                         // Not stored in database
                dcterms_subject: MaybeMultiple::None, // Not stored in database
                ephemeral: Some(Ephemeral {
                    timestamp: Some(object.created_at),
                    metadata: object.ek_metadata.and_then(from_serde),
                    internal_uuid: object.ek_uuid.clone(),
                    ..Default::default()
                }),
                instrument: object.ek_instrument.clone().into(),
                context: None, // ActivityPub context, not stored
            })
        } else {
            Err(anyhow!("ObjectType is not Article"))
        }
    }
}

impl TryFrom<Object> for ApObject {
    type Error = anyhow::Error;

    fn try_from(object: Object) -> Result<Self> {
        match object.as_type {
            ObjectType::Note => Ok(ApObject::Note(object.try_into()?)),
            ObjectType::Article => Ok(ApObject::Article(object.try_into()?)),
            ObjectType::Question => Ok(ApObject::Question(object.try_into()?)),
            _ => Err(anyhow!("unimplemented Object -> ApObject conversion")),
        }
    }
}

impl From<Object> for Vec<ApHashtag> {
    fn from(object: Object) -> Self {
        match ApObject::try_from(object) {
            Ok(ApObject::Note(note)) => note.into(),
            Ok(ApObject::Article(article)) => article.into(),
            _ => vec![],
        }
    }
}

impl TryFrom<Object> for ApQuestion {
    type Error = anyhow::Error;

    fn try_from(object: Object) -> Result<Self, Self::Error> {
        Ok(ApQuestion {
            id: object.as_id,
            attributed_to: from_serde(object.as_attributed_to.ok_or(anyhow!("no attributed_to"))?)
                .ok_or(anyhow!("failed to convert from Value"))?,
            to: from_serde(object.as_to.ok_or(anyhow!("as_to is None"))?)
                .ok_or(anyhow!("failed to deserialize as_to"))?,
            cc: object.as_cc.into(),
            end_time: object.as_end_time.map(ApDateTime::from),
            published: object.as_published.map(ApDateTime::from),
            one_of: object.as_one_of.into(),
            any_of: object.as_any_of.into(),
            content: object.as_content,
            content_map: object.as_content_map.and_then(from_serde),
            summary: object.as_summary,
            voters_count: object.ap_voters_count,
            url: object.as_url.and_then(from_serde),
            conversation: object.ap_conversation,
            tag: object.as_tag.into(),
            attachment: object.as_attachment.into(),
            sensitive: object.ap_sensitive,
            in_reply_to: object.as_in_reply_to.into(),
            ephemeral: Some(Ephemeral {
                created_at: Some(object.created_at),
                updated_at: Some(object.updated_at),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

pub async fn create_or_update_object<C: DbRunner>(conn: &C, object: NewObject) -> Result<Object> {
    conn.run(move |c| {
        diesel::insert_into(objects::table)
            .values(&object)
            .on_conflict(objects::as_id)
            .do_update()
            .set(&object)
            .get_result(c)
    })
    .await
}

pub async fn update_metadata<C: DbRunner>(conn: &C, id: i32, metadata: Value) -> Result<Object> {
    conn.run(move |c| {
        diesel::update(objects::table.filter(objects::id.eq(id)))
            .set(objects::ek_metadata.eq(Some(metadata)))
            .get_result::<Object>(c)
    })
    .await
}

pub async fn update_hashtags<C: DbRunner>(conn: &C, id: i32, hashtags: Value) -> Result<Object> {
    conn.run(move |c| {
        diesel::update(objects::table.filter(objects::id.eq(id)))
            .set(objects::ek_hashtags.eq(hashtags))
            .get_result::<Object>(c)
    })
    .await
}

pub async fn get_object<C: DbRunner>(conn: &C, id: i32) -> Result<Object> {
    conn.run(move |c| objects::table.find(id).first::<Object>(c))
        .await
}

pub async fn get_object_by_as_id<C: DbRunner>(conn: &C, as_id: String) -> Result<Object> {
    conn.run(move |c| {
        objects::table
            .filter(objects::as_id.eq(as_id))
            .first::<Object>(c)
    })
    .await
}

pub async fn get_object_by_uuid<C: DbRunner>(conn: &C, uuid: String) -> Result<Object> {
    conn.run(move |c| {
        objects::table
            .filter(objects::ek_uuid.eq(uuid))
            .first::<Object>(c)
    })
    .await
}

pub async fn tombstone_object_by_as_id<C: DbRunner>(conn: &C, as_id: String) -> Result<Object> {
    conn.run(move |c| {
        diesel::update(objects::table.filter(objects::as_id.eq(as_id)))
            .set(objects::as_type.eq(ObjectType::Tombstone))
            .get_result(c)
    })
    .await
}

pub async fn tombstone_object_by_uuid<C: DbRunner>(conn: &C, uuid: String) -> Result<Object> {
    conn.run(move |c| {
        diesel::update(objects::table.filter(objects::ek_uuid.eq(uuid)))
            .set(objects::as_type.eq(ObjectType::Tombstone))
            .get_result(c)
    })
    .await
}

pub async fn delete_object_by_as_id<C: DbRunner>(conn: &C, as_id: String) -> Result<usize> {
    conn.run(move |c| diesel::delete(objects::table.filter(objects::as_id.eq(as_id))).execute(c))
        .await
}

pub async fn delete_object_by_uuid<C: DbRunner>(conn: &C, uuid: String) -> Result<usize> {
    conn.run(move |c| diesel::delete(objects::table.filter(objects::ek_uuid.eq(uuid))).execute(c))
        .await
}

pub async fn delete_objects_by_domain_pattern<C: DbRunner>(
    conn: &C,
    domain_pattern: String,
) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("DELETE FROM objects WHERE as_attributed_to::text LIKE $1")
            .bind::<Text, _>(format!("\"https://{domain_pattern}/%\""))
            .execute(c)
    };

    conn.run(operation).await
}

pub async fn delete_objects_by_attributed_to<C: DbRunner>(
    conn: &C,
    attributed_to: String,
) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Jsonb;

        sql_query("DELETE FROM objects WHERE as_attributed_to @> $1")
            .bind::<Jsonb, _>(json!(attributed_to))
            .execute(c)
    };

    conn.run(operation).await
}

// pub async fn delete_objects_by_domain_pattern(
//     conn: Option<&Db>,
//     domain_pattern: String,
// ) -> Result<usize> {
//     let operation = move |c: &mut diesel::PgConnection| {
//         use diesel::sql_types::Text;

//         sql_query("DELETE FROM objects WHERE as_attributed_to::text LIKE $1")
//             .bind::<Text, _>(format!("\"https://{domain_pattern}/%\""))
//             .execute(c)
//     };

//     crate::db::run_db_op(conn, &crate::POOL, operation).await
// }

// pub async fn delete_objects_by_attributed_to(
//     conn: Option<&Db>,
//     attributed_to: String,
// ) -> Result<usize> {
//     let operation = move |c: &mut diesel::PgConnection| {
//         use diesel::sql_types::Jsonb;

//         sql_query("DELETE FROM objects WHERE as_attributed_to @> $1")
//             .bind::<Jsonb, _>(json!(attributed_to))
//             .execute(c)
//     };

//     crate::db::run_db_op(conn, &crate::POOL, operation).await
// }
