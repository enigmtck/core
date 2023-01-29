use crate::activity_pub::ApNote;
use crate::db::Db;
use crate::schema::{timeline, timeline_cc, timeline_to};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
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
    pub summary: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub atom_uri: Option<String>,
    pub in_reply_to_atom_uri: Option<String>,
    pub conversation: Option<String>,
    pub content_map: Option<Value>,
    pub attachment: Option<Value>,
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
            summary: note.0.summary,
            ap_sensitive: note.0.ap_sensitive,
            atom_uri: note.0.atom_uri,
            in_reply_to_atom_uri: note.0.in_reply_to_atom_uri,
            conversation: note.0.conversation,
            content_map: note.0.content_map,
            attachment: note.0.attachment,
        }
    }
}

type IdentifiedApNote = (ApNote, i32);

impl From<IdentifiedApNote> for NewTimelineItem {
    fn from(inote: IdentifiedApNote) -> Self {
        let note = inote.0;
        let remote_actor_id = inote.1;

        let ap_public = {
            note.to
                .contains(&"https://www.w3.org/ns/activitystreams#Public".to_string())
        };

        NewTimelineItem {
            tag: Option::from(serde_json::to_value(&note.tag).unwrap_or_default()),
            attributed_to: note.attributed_to,
            remote_actor_id,
            ap_id: note.id.unwrap(),
            kind: note.kind.to_string(),
            url: note.url,
            published: note.published,
            replies: Option::from(serde_json::to_value(&note.replies).unwrap_or_default()),
            in_reply_to: note.in_reply_to,
            content: note.content,
            ap_public,
            summary: note.summary,
            ap_sensitive: note.sensitive,
            atom_uri: note.atom_uri,
            in_reply_to_atom_uri: note.in_reply_to_atom_uri,
            conversation: note.conversation,
            content_map: note.content_map,
            attachment: Option::from(serde_json::to_value(&note.attachment).unwrap_or_default()),
            ..Default::default()
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
    pub summary: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub atom_uri: Option<String>,
    pub in_reply_to_atom_uri: Option<String>,
    pub conversation: Option<String>,
    pub content_map: Option<Value>,
    pub attachment: Option<Value>,
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

pub async fn get_timeline_items_by_ap_id_paged(
    conn: &Db,
    ap_id: String,
    limit: i64,
    offset: i64,
) -> Vec<TimelineItem> {
    match conn
        .run(move |c| {
            let query = timeline::table
                .filter(timeline::ap_public.eq(true))
                .order(timeline::created_at.desc())
                .limit(limit)
                .offset(offset)
                .into_boxed();

            query.get_results::<TimelineItem>(c)
        })
        .await
    {
        Ok(x) => x,
        Err(_) => vec![],
    }
}
