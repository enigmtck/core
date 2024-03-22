use crate::db::Db;
use crate::schema::notifications;
use diesel::prelude::*;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::notifications::NotificationType;
        pub use crate::models::pg::notifications::Notification;
        pub use crate::models::pg::notifications::NewNotification;
        pub use crate::models::pg::notifications::_create_notification;
        pub use crate::models::pg::notifications::_delete_by_filter;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::notifications::NotificationType;
        pub use crate::models::sqlite::notifications::Notification;
        pub use crate::models::sqlite::notifications::NewNotification;
        pub use crate::models::sqlite::notifications::_create_notification;
        pub use crate::models::sqlite::notifications::_delete_by_filter;
    }
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
