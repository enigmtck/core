use crate::activity_pub::{ApActivity, ApNote};
use crate::db::Db;
use crate::schema::{timeline, timeline_cc, timeline_to};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::remote_notes::RemoteNote;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone, AsChangeset)]
#[table_name = "timeline"]
pub struct NewTimelineItem {
    pub tag: Option<Value>,
    pub attributed_to: String,
    pub ap_id: String,
    pub kind: String,
    pub url: Option<String>,
    pub published: Option<String>,
    pub replies: Option<Value>,
    pub in_reply_to: Option<String>,
    pub content: Option<String>,
    pub ap_public: bool,
    pub summary: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub atom_uri: Option<String>,
    pub in_reply_to_atom_uri: Option<String>,
    pub conversation: Option<String>,
    pub content_map: Option<Value>,
    pub attachment: Option<Value>,
    pub ap_object: Option<Value>,
    pub announce: Option<String>,
}

impl From<RemoteNote> for NewTimelineItem {
    fn from(note: RemoteNote) -> Self {
        NewTimelineItem {
            tag: note.clone().tag,
            attributed_to: note.clone().attributed_to,
            ap_id: note.clone().ap_id,
            kind: note.clone().kind,
            url: note.clone().url,
            published: note.clone().published,
            replies: note.clone().replies,
            in_reply_to: note.clone().in_reply_to,
            content: Option::from(note.clone().content),
            ap_public: note.is_public(),
            summary: note.summary,
            ap_sensitive: note.ap_sensitive,
            atom_uri: note.atom_uri,
            in_reply_to_atom_uri: note.in_reply_to_atom_uri,
            conversation: note.conversation,
            content_map: note.content_map,
            attachment: note.attachment,
            ap_object: Option::None,
            announce: Option::None,
        }
    }
}

impl From<ApNote> for NewTimelineItem {
    fn from(note: ApNote) -> Self {
        NewTimelineItem {
            tag: Option::from(serde_json::to_value(&note.tag).unwrap_or_default()),
            attributed_to: note.clone().attributed_to,
            ap_id: note.clone().id.unwrap(),
            kind: note.kind.to_string(),
            url: note.clone().url,
            published: note.clone().published,
            replies: Option::from(serde_json::to_value(&note.replies).unwrap_or_default()),
            in_reply_to: note.clone().in_reply_to,
            content: Option::from(note.clone().content),
            ap_public: note.is_public(),
            summary: note.summary,
            ap_sensitive: note.sensitive,
            atom_uri: note.atom_uri,
            in_reply_to_atom_uri: note.in_reply_to_atom_uri,
            conversation: note.conversation,
            content_map: Option::from(serde_json::to_value(&note.content_map).unwrap_or_default()),
            attachment: Option::from(serde_json::to_value(&note.attachment).unwrap_or_default()),
            ap_object: Option::None,
            announce: Option::None,
        }
    }
}

type Announce = (ApActivity, ApNote);

impl From<Announce> for NewTimelineItem {
    fn from((activity, note): Announce) -> Self {
        NewTimelineItem {
            tag: Option::from(serde_json::to_value(&note.tag).unwrap_or_default()),
            attributed_to: note.clone().attributed_to,
            ap_id: note.clone().id.unwrap(),
            kind: note.kind.to_string(),
            url: note.clone().url,
            published: activity.published,
            replies: Option::from(serde_json::to_value(&note.replies).unwrap_or_default()),
            in_reply_to: note.clone().in_reply_to,
            content: Option::from(note.clone().content),
            ap_public: note.is_public(),
            summary: note.summary,
            ap_sensitive: note.sensitive,
            atom_uri: note.atom_uri,
            in_reply_to_atom_uri: note.in_reply_to_atom_uri,
            conversation: note.conversation,
            content_map: Option::from(
                serde_json::to_value(note.content_map.unwrap_or_default()).unwrap_or_default(),
            ),
            attachment: Option::from(
                serde_json::to_value(note.attachment.unwrap_or_default()).unwrap_or_default(),
            ),
            ap_object: Option::None,
            announce: Option::from(activity.actor),
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
    pub ap_id: String,
    pub kind: String,
    pub url: Option<String>,
    pub published: Option<String>,
    pub replies: Option<Value>,
    pub in_reply_to: Option<String>,
    pub content: Option<String>,
    pub ap_public: bool,
    pub summary: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub atom_uri: Option<String>,
    pub in_reply_to_atom_uri: Option<String>,
    pub conversation: Option<String>,
    pub content_map: Option<Value>,
    pub attachment: Option<Value>,
    pub ap_object: Option<Value>,
    pub announce: Option<String>,
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

pub async fn get_public_timeline_items(conn: &Db, limit: i64, offset: i64) -> Vec<TimelineItem> {
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

pub async fn get_timeline_items_by_conversation(
    conn: &Db,
    conversation: String,
    limit: i64,
    offset: i64,
) -> Vec<TimelineItem> {
    match conn
        .run(move |c| {
            let query = timeline::table
                .filter(timeline::conversation.eq(conversation))
                .order(timeline::created_at.asc())
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

pub async fn delete_timeline_item_by_ap_id(conn: &Db, ap_id: String) -> Result<(), ()> {
    use crate::schema::timeline::dsl::{ap_id as a, timeline};

    match conn
        .run(move |c| diesel::delete(timeline.filter(a.eq(ap_id))).execute(c))
        .await
    {
        Ok(_) => Ok(()),
        Err(_) => Err(()),
    }
}
