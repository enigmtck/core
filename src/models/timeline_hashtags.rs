use crate::activity_pub::{ApNote, ApTag};
use crate::db::Db;
use crate::schema::timeline_hashtags;
use crate::POOL;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

use super::timeline::TimelineItem;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = timeline_hashtags)]
pub struct NewTimelineHashtag {
    pub hashtag: String,
    pub timeline_id: i32,
}

impl From<TimelineItem> for Vec<NewTimelineHashtag> {
    fn from(timeline_item: TimelineItem) -> Self {
        let ap_note: ApNote = timeline_item.clone().into();

        ap_note
            .tag
            .unwrap_or_default()
            .iter()
            .filter_map(|tag| {
                if let ApTag::HashTag(tag) = tag {
                    Some(NewTimelineHashtag {
                        hashtag: tag.name.clone(),
                        timeline_id: timeline_item.id,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

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
