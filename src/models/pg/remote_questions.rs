use crate::activity_pub::ApQuestionType;
use crate::db::Db;
use crate::schema::remote_questions;
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
#[ExistingTypePath = "crate::schema::sql_types::QuestionType"]
pub enum QuestionType {
    #[default]
    Question,
}

impl fmt::Display for QuestionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<ApQuestionType> for QuestionType {
    fn from(kind: ApQuestionType) -> Self {
        match kind {
            ApQuestionType::Question => QuestionType::Question,
        }
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, AsChangeset)]
#[diesel(table_name = remote_questions)]
pub struct NewRemoteQuestion {
    pub kind: QuestionType,
    pub ap_id: String,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub end_time: Option<DateTime<Utc>>,
    pub published: Option<DateTime<Utc>>,
    pub one_of: Option<Value>,
    pub any_of: Option<Value>,
    pub content: Option<String>,
    pub content_map: Option<Value>,
    pub summary: Option<String>,
    pub voters_count: Option<i32>,
    pub url: Option<String>,
    pub conversation: Option<String>,
    pub tag: Option<Value>,
    pub attachment: Option<Value>,
    pub ap_sensitive: Option<bool>,
    pub in_reply_to: Option<String>,
    pub attributed_to: String,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Default, Debug)]
#[diesel(table_name = remote_questions)]
pub struct RemoteQuestion {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub kind: QuestionType,
    pub ap_id: String,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub end_time: Option<DateTime<Utc>>,
    pub published: Option<DateTime<Utc>>,
    pub one_of: Option<Value>,
    pub any_of: Option<Value>,
    pub content: Option<String>,
    pub content_map: Option<Value>,
    pub summary: Option<String>,
    pub voters_count: Option<i32>,
    pub url: Option<String>,
    pub conversation: Option<String>,
    pub tag: Option<Value>,
    pub attachment: Option<Value>,
    pub ap_sensitive: Option<bool>,
    pub in_reply_to: Option<String>,
    pub attributed_to: String,
}

pub async fn create_or_update_remote_question(
    conn: &Db,
    remote_question: NewRemoteQuestion,
) -> Result<RemoteQuestion> {
    conn.run(move |c| {
        diesel::insert_into(remote_questions::table)
            .values(&remote_question)
            .on_conflict(remote_questions::ap_id)
            .do_update()
            .set(&remote_question)
            .get_result(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}
