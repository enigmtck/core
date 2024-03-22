use crate::db::Db;
use crate::models::timeline_hashtags::NewTimelineHashtag;
use crate::schema::timeline_hashtags;
use crate::POOL;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::Serialize;

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = timeline_hashtags)]
pub struct TimelineHashtag {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub hashtag: String,
    pub timeline_id: i32,
}

pub async fn create_timeline_hashtag(
    conn: Option<&Db>,
    hashtag: NewTimelineHashtag,
) -> Option<TimelineHashtag> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(timeline_hashtags::table)
                    .values(&hashtag)
                    .execute(c)
                    .ok()?;

                timeline_hashtags::table
                    .order(timeline_hashtags::id.desc())
                    .first::<TimelineHashtag>(c)
                    .ok()
            })
            .await
        }
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(timeline_hashtags::table)
                .values(&hashtag)
                .execute(&mut pool)
                .ok()?;

            timeline_hashtags::table
                .order(timeline_hashtags::id.desc())
                .first::<TimelineHashtag>(&mut pool)
                .ok()
        }
    }
}
