use crate::db::Db;
use crate::schema::notifications;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::query_builder::QueryId;
use diesel::sql_types::Bool;
use diesel::sqlite::Sqlite;
use diesel::{AsChangeset, Identifiable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq)]
pub enum NotificationType {
    #[default]
    Mention,
    Announce,
    Unannounce,
    Like,
    Unlike,
    Follow,
    Unfollow,
    Accept,
    Block,
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = notifications)]
pub struct NewNotification {
    pub uuid: String,
    pub kind: String,
    pub profile_id: i32,
    pub activity_id: i32,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = notifications)]
pub struct Notification {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub uuid: String,
    pub kind: String,
    pub profile_id: i32,
    pub activity_id: i32,
}

pub async fn _create_notification(
    conn: &Db,
    notification: NewNotification,
) -> Option<Notification> {
    conn.run(move |c| {
        diesel::insert_into(notifications::table)
            .values(&notification)
            .execute(c)?;

        notifications::table
            .order(notifications::id.desc())
            .first::<Notification>(c)
    })
    .await
    .ok()
}

pub async fn _delete_by_filter<T>(conn: &Db, filter: T) -> bool
where
    T: diesel::BoxableExpression<notifications::table, Sqlite, SqlType = Bool>
        + QueryId
        + Send
        + 'static,
{
    conn.run(move |c| {
        diesel::delete(notifications::table)
            .filter(filter)
            .execute(c)
    })
    .await
    .is_ok()
}
