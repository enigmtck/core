use crate::db::Db;
use crate::models::timeline_hashtags::NewTimelineHashtag;
use crate::schema::timeline_hashtags;
use crate::POOL;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

use super::timeline::JoinedTimelineItem;

#[derive(
    Identifiable,
    Queryable,
    AsChangeset,
    Serialize,
    Clone,
    Default,
    Debug,
    QueryableByName,
    Deserialize,
)]
#[diesel(table_name = timeline_hashtags)]
pub struct TimelineHashtag {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub hashtag: String,
    pub timeline_id: i32,
}

impl TryFrom<JoinedTimelineItem> for TimelineHashtag {
    type Error = anyhow::Error;

    fn try_from(item: JoinedTimelineItem) -> Result<Self, Self::Error> {
        let id = item
            .timeline_hashtag_id
            .ok_or_else(|| anyhow::Error::msg("missing id"))?;
        let created_at = item
            .timeline_hashtag_created_at
            .ok_or_else(|| anyhow::Error::msg("missing created_at"))?;
        let updated_at = item
            .timeline_hashtag_updated_at
            .ok_or_else(|| anyhow::Error::msg("missing updated_at"))?;
        let hashtag = item
            .timeline_hashtag_hashtag
            .ok_or_else(|| anyhow::Error::msg("missing hashtag"))?;
        let timeline_id = item
            .timeline_hashtag_timeline_id
            .ok_or_else(|| anyhow::Error::msg("missing timeline_id"))?;

        Ok(TimelineHashtag {
            id,
            created_at,
            updated_at,
            hashtag,
            timeline_id,
        })
    }
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
