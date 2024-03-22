use crate::db::Db;
use crate::models::timeline_hashtags::NewTimelineHashtag;
use crate::schema::timeline_hashtags;
use crate::POOL;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::Serialize;

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = timeline_hashtags)]
pub struct TimelineHashtag {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub hashtag: String,
    pub timeline_id: i32,
}

pub async fn create_timeline_hashtag(
    conn: Option<&Db>,
    hashtag: NewTimelineHashtag,
) -> Option<TimelineHashtag> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(timeline_hashtags::table)
                    .values(&hashtag)
                    .get_result::<TimelineHashtag>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(timeline_hashtags::table)
                .values(&hashtag)
                .get_result::<TimelineHashtag>(&mut pool)
                .ok()
        }
    }
}
