use crate::db::Db;
use crate::schema::notifications;
use chrono::{DateTime, Utc};
use convert_case::{Case, Casing};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::QueryId;
use diesel::sql_types::Bool;
use diesel::{AsChangeset, Identifiable, Queryable};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};

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

impl fmt::Display for NotificationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl From<NotificationType> for String {
    fn from(notification: NotificationType) -> Self {
        format!("{notification}").to_case(Case::Snake)
    }
}

impl From<String> for NotificationType {
    fn from(notification: String) -> Self {
        match notification.to_case(Case::Snake).as_str() {
            "mention" => NotificationType::Mention,
            "announce" => NotificationType::Announce,
            "unannounce" => NotificationType::Unannounce,
            "like" => NotificationType::Like,
            "unlike" => NotificationType::Unlike,
            "follow" => NotificationType::Follow,
            "unfollow" => NotificationType::Unfollow,
            "accept" => NotificationType::Accept,
            "block" => NotificationType::Block,
            _ => NotificationType::Mention,
        }
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = notifications)]
pub struct NewNotification {
    pub uuid: String,

    #[cfg(feature = "pg")]
    pub kind: NotificationType,

    #[cfg(feature = "sqlite")]
    pub kind: String,

    pub profile_id: i32,
    pub activity_id: i32,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = notifications)]
pub struct Notification {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,

    #[cfg(feature = "pg")]
    pub kind: NotificationType,

    #[cfg(feature = "sqlite")]
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

pub async fn _get_notification_by_uuid(conn: &Db, uuid: String) -> Option<Notification> {
    conn.run(move |c| {
        notifications::table
            .filter(notifications::uuid.eq(uuid))
            .first::<Notification>(c)
    })
    .await
    .ok()
}

pub async fn _delete_notification(conn: &Db, id: i32) -> bool {
    _delete_by_filter(conn, notifications::id.eq(id)).await
}

pub async fn _delete_notification_by_uuid(conn: &Db, uuid: String) -> bool {
    _delete_by_filter(conn, notifications::uuid.eq(uuid)).await
}
