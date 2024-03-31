use crate::db::Db;
use crate::schema::remote_questions;
use anyhow::Result;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Insertable, Default, Debug, AsChangeset)]
#[diesel(table_name = remote_questions)]
pub struct NewRemoteQuestion {
    pub kind: String,
    pub ap_id: String,
    pub ap_to: Option<String>,
    pub cc: Option<String>,
    pub end_time: Option<NaiveDateTime>,
    pub published: Option<NaiveDateTime>,
    pub one_of: Option<String>,
    pub any_of: Option<String>,
    pub content: Option<String>,
    pub content_map: Option<String>,
    pub summary: Option<String>,
    pub voters_count: Option<i32>,
    pub url: Option<String>,
    pub conversation: Option<String>,
    pub tag: Option<String>,
    pub attachment: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub in_reply_to: Option<String>,
    pub attributed_to: String,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Default, Debug)]
#[diesel(table_name = remote_questions)]
pub struct RemoteQuestion {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: Option<String>,
    pub cc: Option<String>,
    pub end_time: Option<NaiveDateTime>,
    pub published: Option<NaiveDateTime>,
    pub one_of: Option<String>,
    pub any_of: Option<String>,
    pub content: Option<String>,
    pub content_map: Option<String>,
    pub summary: Option<String>,
    pub voters_count: Option<i32>,
    pub url: Option<String>,
    pub conversation: Option<String>,
    pub tag: Option<String>,
    pub attachment: Option<String>,
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
            .execute(c)?;

        remote_questions::table
            .filter(remote_questions::ap_id.eq(&remote_question.ap_id))
            .first::<RemoteQuestion>(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}
