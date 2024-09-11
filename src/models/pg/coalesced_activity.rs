use crate::models::pg::activities::ActivityType;
use crate::schema::sql_types::ActivityType as SqlActivityType;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::{Bool, Integer, Jsonb, Nullable, Text, Timestamptz};
use diesel::Queryable;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Queryable, Serialize, Deserialize, Clone, Default, Debug, QueryableByName)]
pub struct CoalescedActivity {
    // Activity Fields
    #[sql_type = "Integer"]
    pub id: i32,

    #[sql_type = "Timestamptz"]
    pub created_at: DateTime<Utc>,

    #[sql_type = "Timestamptz"]
    pub updated_at: DateTime<Utc>,

    #[sql_type = "Nullable<Integer>"]
    pub profile_id: Option<i32>,

    #[sql_type = "SqlActivityType"]
    pub kind: ActivityType,

    #[sql_type = "Text"]
    pub uuid: String,

    #[sql_type = "Text"]
    pub actor: String,

    #[sql_type = "Nullable<Jsonb>"]
    pub ap_to: Option<Value>,

    #[sql_type = "Nullable<Jsonb>"]
    pub cc: Option<Value>,

    #[sql_type = "Nullable<Integer>"]
    pub target_note_id: Option<i32>,

    #[sql_type = "Nullable<Integer>"]
    pub target_remote_note_id: Option<i32>,

    #[sql_type = "Nullable<Integer>"]
    pub target_profile_id: Option<i32>,

    #[sql_type = "Nullable<Integer>"]
    pub target_activity_id: Option<i32>,

    #[sql_type = "Nullable<Text>"]
    pub target_ap_id: Option<String>,

    #[sql_type = "Nullable<Integer>"]
    pub target_remote_actor_id: Option<i32>,

    #[sql_type = "Bool"]
    pub revoked: bool,

    #[sql_type = "Nullable<Text>"]
    pub ap_id: Option<String>,

    #[sql_type = "Nullable<Integer>"]
    pub target_remote_question_id: Option<i32>,

    #[sql_type = "Bool"]
    pub reply: bool,

    // Object Fields
    #[sql_type = "Nullable<Text>"]
    pub object_uuid: Option<String>,

    #[sql_type = "Nullable<Text>"]
    pub object_type: Option<String>,

    #[sql_type = "Nullable<Text>"]
    pub object_published: Option<String>,

    #[sql_type = "Nullable<Text>"]
    pub object_id: Option<String>,

    #[sql_type = "Nullable<Text>"]
    pub object_url: Option<String>,

    #[sql_type = "Nullable<Jsonb>"]
    pub object_to: Option<Value>,

    #[sql_type = "Nullable<Jsonb>"]
    pub object_cc: Option<Value>,

    #[sql_type = "Nullable<Jsonb>"]
    pub object_tag: Option<Value>,

    #[sql_type = "Nullable<Text>"]
    pub object_attributed_to: Option<String>,

    #[sql_type = "Nullable<Text>"]
    pub object_in_reply_to: Option<String>,

    #[sql_type = "Nullable<Text>"]
    pub object_content: Option<String>,

    #[sql_type = "Nullable<Text>"]
    pub object_conversation: Option<String>,

    #[sql_type = "Nullable<Jsonb>"]
    pub object_attachment: Option<Value>,

    #[sql_type = "Nullable<Text>"]
    pub object_summary: Option<String>,

    #[sql_type = "Nullable<Timestamptz>"]
    pub object_end_time: Option<DateTime<Utc>>,

    #[sql_type = "Nullable<Jsonb>"]
    pub object_one_of: Option<Value>,

    #[sql_type = "Nullable<Jsonb>"]
    pub object_any_of: Option<Value>,

    #[sql_type = "Nullable<Integer>"]
    pub object_voters_count: Option<i32>,

    #[sql_type = "Nullable<Bool>"]
    pub object_sensitive: Option<bool>,
}
