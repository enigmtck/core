use crate::models::pg::activities::ActivityType;
use crate::models::pg::objects::ObjectType;
use crate::schema::sql_types::{ActivityType as SqlActivityType, ObjectType as SqlObjectType};
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

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub raw: Option<Value>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub target_object_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub actor_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub target_actor_id: Option<i32>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub recursive_created_at: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub recursive_updated_at: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub recursive_profile_id: Option<i32>,

    #[diesel(sql_type = Nullable<SqlActivityType>)]
    pub recursive_kind: Option<ActivityType>,

    #[diesel(sql_type = Nullable<Text>)]
    pub recursive_uuid: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub recursive_actor: Option<String>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub recursive_ap_to: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub recursive_cc: Option<Value>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub recursive_target_note_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub recursive_target_remote_note_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub recursive_target_profile_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub recursive_target_activity_id: Option<i32>,

    #[diesel(sql_type = Nullable<Text>)]
    pub recursive_target_ap_id: Option<String>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub recursive_target_remote_actor_id: Option<i32>,

    #[diesel(sql_type = Nullable<Bool>)]
    pub recursive_revoked: Option<bool>,

    #[diesel(sql_type = Nullable<Text>)]
    pub recursive_ap_id: Option<String>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub recursive_target_remote_question_id: Option<i32>,

    #[diesel(sql_type = Nullable<Bool>)]
    pub recursive_reply: Option<bool>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub recursive_target_object_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub recursive_actor_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub recursive_target_actor_id: Option<i32>,

    // Object Fields
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub object_created_at: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub object_updated_at: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_uuid: Option<String>,

    #[diesel(sql_type = Nullable<SqlObjectType>)]
    pub object_type: Option<ObjectType>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub object_published: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_as_id: Option<String>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_url: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_to: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_cc: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_tag: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_attributed_to: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_in_reply_to: Option<Value>,

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

    #[diesel(sql_type = Nullable<Integer>)]
    pub object_profile_id: Option<i32>,

    #[diesel(sql_type = Jsonb)]
    pub object_announcers: Value,

    #[diesel(sql_type = Jsonb)]
    pub object_likers: Value,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_announced: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_liked: Option<String>,
}
