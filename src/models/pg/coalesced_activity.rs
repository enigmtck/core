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
    #[diesel(sql_type = Integer)]
    pub id: i32,

    #[diesel(sql_type = Timestamptz)]
    pub created_at: DateTime<Utc>,

    #[diesel(sql_type = Timestamptz)]
    pub updated_at: DateTime<Utc>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub profile_id: Option<i32>,

    #[diesel(sql_type = SqlActivityType)]
    pub kind: ActivityType,

    #[diesel(sql_type = Text)]
    pub uuid: String,

    #[diesel(sql_type = Text)]
    pub actor: String,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub ap_to: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub cc: Option<Value>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub target_note_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub target_remote_note_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub target_profile_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub target_activity_id: Option<i32>,

    #[diesel(sql_type = Nullable<Text>)]
    pub target_ap_id: Option<String>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub target_remote_actor_id: Option<i32>,

    #[diesel(sql_type = Bool)]
    pub revoked: bool,

    #[diesel(sql_type = Nullable<Text>)]
    pub ap_id: Option<String>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub target_remote_question_id: Option<i32>,

    #[diesel(sql_type = Bool)]
    pub reply: bool,

    // Object Fields
    #[diesel(sql_type = Nullable<Text>)]
    pub object_uuid: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_type: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_published: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_id: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_url: Option<String>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_to: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_cc: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_tag: Option<Value>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_attributed_to: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_in_reply_to: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_content: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_conversation: Option<String>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_attachment: Option<Value>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_summary: Option<String>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub object_end_time: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_one_of: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_any_of: Option<Value>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub object_voters_count: Option<i32>,

    #[diesel(sql_type = Nullable<Bool>)]
    pub object_sensitive: Option<bool>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_metadata: Option<Value>,

    #[diesel(sql_type = Jsonb)]
    pub object_announcers: Value,

    #[diesel(sql_type = Jsonb)]
    pub object_likers: Value,
}
