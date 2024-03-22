use crate::db::Db;
use crate::models::notifications::NewNotification;
use crate::schema::notifications;
use chrono::{DateTime, Utc};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::QueryId;
use diesel::sql_types::Bool;
use diesel::{AsChangeset, Identifiable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(
    diesel_derive_enum::DbEnum, Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq,
)]
#[ExistingTypePath = "crate::schema::sql_types::NotificationType"]
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

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = notifications)]
pub struct Notification {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub kind: NotificationType,
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
            .get_result::<Notification>(c)
    })
    .await
    .ok()
}

pub async fn _delete_by_filter<T>(conn: &Db, filter: T) -> bool
where
    T: diesel::BoxableExpression<notifications::table, Pg, SqlType = Bool>
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
