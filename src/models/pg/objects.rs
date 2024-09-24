use crate::db::Db;
use crate::schema::objects;
use crate::POOL;
use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

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
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ObjectType {
    pub fn is_note(&self) -> bool {
        match self {
            ObjectType::Note => true,
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, AsChangeset)]
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
    pub as_in_reply_to: Option<Value>,
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
    pub as_type: ObjectType,
    pub as_updated: Option<DateTime<Utc>>,
    pub as_url: Option<Value>,
    pub ek_instrument: Option<Value>,
    pub ek_metadata: Option<Value>,
    pub ek_profile_id: Option<i32>,
    pub ek_uuid: Option<String>,
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
    pub as_in_reply_to: Option<Value>,
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
    pub as_type: ObjectType,
    pub as_updated: Option<DateTime<Utc>>,
    pub as_url: Option<Value>,
    pub ek_hashtags: Value,
    pub ek_instrument: Option<Value>,
    pub ek_metadata: Option<Value>,
    pub ek_profile_id: Option<i32>,
    pub ek_uuid: Option<String>,
}

pub async fn create_or_update_object(conn: &Db, object: NewObject) -> Result<Object> {
    conn.run(move |c| {
        diesel::insert_into(objects::table)
            .values(&object)
            .on_conflict(objects::as_id)
            .do_update()
            .set(&object)
            .get_result(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn update_metadata(conn: &Db, id: i32, metadata: Value) -> Result<Object> {
    conn.run(move |c| {
        diesel::update(objects::table.filter(objects::id.eq(id)))
            .set(objects::ek_metadata.eq(Some(metadata)))
            .get_result::<Object>(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn update_hashtags(conn: &Db, id: i32, hashtags: Value) -> Result<Object> {
    conn.run(move |c| {
        diesel::update(objects::table.filter(objects::id.eq(id)))
            .set(objects::ek_hashtags.eq(hashtags))
            .get_result::<Object>(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}
