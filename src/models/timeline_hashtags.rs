use crate::activity_pub::{ApNote, ApTag};
use crate::schema::timeline_hashtags;
use diesel::Insertable;
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

use super::timeline::TimelineItem;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::timeline_hashtags::TimelineHashtag;
        pub use crate::models::pg::timeline_hashtags::create_timeline_hashtag;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::timeline_hashtags::TimelineHashtag;
        pub use crate::models::sqlite::timeline_hashtags::create_timeline_hashtag;
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = timeline_hashtags)]
pub struct NewTimelineHashtag {
    pub hashtag: String,
    pub timeline_id: i32,
}

impl TryFrom<TimelineItem> for Vec<NewTimelineHashtag> {
    type Error = anyhow::Error;
    fn try_from(timeline_item: TimelineItem) -> Result<Self, Self::Error> {
        let ap_note: ApNote = timeline_item.clone().try_into()?;

        Ok(ap_note
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
            .collect())
    }
}
