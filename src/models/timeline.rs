use crate::schema::{timeline, timeline_cc, timeline_to};
use crate::db::Db;
use diesel::prelude::*;
use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::remote_notes::RemoteNote;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[table_name = "timeline"]
pub struct NewTimelineItem {
    pub tag: Option<Value>,
    pub attributed_to: String,
    pub remote_actor_id: i32,
    pub ap_id: String,
    pub kind: String,
    pub url: Option<String>,
    pub published: Option<String>,
    pub replies: Option<Value>,
    pub in_reply_to: Option<String>,
    pub content: String,
    pub ap_public: bool,
}

type IdentifiedRemoteNote = (RemoteNote, i32);

impl From<IdentifiedRemoteNote> for NewTimelineItem {
    fn from(note: IdentifiedRemoteNote) -> Self {
        let ap_public = {
            if let Some(ap_to) = note.0.ap_to {
                if let Ok(ap_to) = serde_json::from_value::<Vec<String>>(ap_to) {
                    ap_to.contains(&"https://www.w3.org/ns/activitystreams#Public".to_string())
                } else {
                    false
                }
            } else {
                false
            }
        };

        NewTimelineItem {
            tag: note.0.tag,
            attributed_to: note.0.attributed_to,
            remote_actor_id: note.1,
            ap_id: note.0.ap_id,
            kind: note.0.kind,
            url: note.0.url,
            published: note.0.published,
            replies: note.0.replies,
            in_reply_to: note.0.in_reply_to,
            content: note.0.content,
            ap_public,
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "timeline"]
pub struct TimelineItem {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tag: Option<Value>,
    pub attributed_to: String,
    pub remote_actor_id: i32,
    pub ap_id: String,
    pub kind: String,
    pub url: Option<String>,
    pub published: Option<String>,
    pub replies: Option<Value>,
    pub in_reply_to: Option<String>,
    pub content: String,
    pub ap_public: bool,
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[table_name = "timeline_cc"]
pub struct NewTimelineItemCc {
    pub timeline_id: i32,
    pub ap_id: String,
}

#[derive(Identifiable, Queryable, AsChangeset, Associations, Serialize, Clone, Default, Debug)]
#[belongs_to(TimelineItem, foreign_key = "timeline_id")]
#[table_name = "timeline_cc"]
pub struct TimelineItemCc {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub timeline_id: i32,
    pub ap_id: String,
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[table_name = "timeline_to"]
pub struct NewTimelineItemTo {
    pub timeline_id: i32,
    pub ap_id: String,
}

#[derive(Identifiable, Queryable, AsChangeset, Associations, Serialize, Clone, Default, Debug)]
#[belongs_to(TimelineItem, foreign_key = "timeline_id")]
#[table_name = "timeline_to"]
pub struct TimelineItemTo {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub timeline_id: i32,
    pub ap_id: String,
}

type IdentifiedTimelineItem = (TimelineItem, String);

impl From<IdentifiedTimelineItem> for NewTimelineItemCc {
    fn from(identified_timeline_item: IdentifiedTimelineItem) -> Self {
        NewTimelineItemCc {
            timeline_id: identified_timeline_item.0.id,
            ap_id: identified_timeline_item.1,
        }
    }
}

impl From<IdentifiedTimelineItem> for NewTimelineItemTo {
    fn from(identified_timeline_item: IdentifiedTimelineItem) -> Self {
        NewTimelineItemTo {
            timeline_id: identified_timeline_item.0.id,
            ap_id: identified_timeline_item.1,
        }
    }
}

pub async fn get_timeline_items_by_ap_id(conn: &Db, ap_id: String)
                                         -> Vec<(TimelineItem,
                                                 Option<TimelineItemCc>,
                                                 Option<TimelineItemTo>)> {
    
    match conn.run(move |c| {
        let query = timeline::table
            .left_join(timeline_cc::table.on(timeline_cc::timeline_id.eq(timeline::id)))
            .left_join(timeline_to::table.on(timeline_to::timeline_id.eq(timeline::id)))
            .filter(timeline::ap_public.eq(true))
            .or_filter(timeline_cc::ap_id.eq(ap_id.clone()))
            .or_filter(timeline_to::ap_id.eq(ap_id))
            .order(timeline::created_at.desc())
            .into_boxed();
        
        query.get_results::<(TimelineItem,
                             Option<TimelineItemCc>,
                             Option<TimelineItemTo>)>(c)}).await {
        Ok(x) => x,
        Err(_) => vec![]
    }
}
