use crate::activity_pub::{ApAnnounce, ApNote};
use crate::db::Db;
use crate::helper::get_ap_id_from_username;
use crate::schema::{
    activities, remote_announces, remote_likes, timeline, timeline_cc, timeline_to,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::activities::Activity;
// use super::announces::Announce;
// use super::likes::Like;
use super::notes::Note;
use super::profiles::Profile;
use super::remote_announces::RemoteAnnounce;
use super::remote_likes::RemoteLike;
use super::remote_notes::RemoteNote;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone, AsChangeset)]
#[diesel(table_name = timeline)]
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
    pub metadata: Option<Value>,
}

impl From<RemoteNote> for NewTimelineItem {
    fn from(note: RemoteNote) -> Self {
        NewTimelineItem {
            ap_public: note.clone().is_public(),
            tag: note.tag,
            attributed_to: note.attributed_to,
            ap_id: note.ap_id,
            kind: note.kind,
            url: note.url,
            published: note.published,
            replies: note.replies,
            in_reply_to: note.in_reply_to,
            content: Some(note.content.clone()),
            summary: note.summary,
            ap_sensitive: note.ap_sensitive,
            atom_uri: note.atom_uri,
            in_reply_to_atom_uri: note.in_reply_to_atom_uri,
            conversation: note.conversation,
            content_map: note.content_map,
            attachment: note.attachment,
            ap_object: None,
            metadata: None,
        }
    }
}

impl From<Note> for NewTimelineItem {
    fn from(note: Note) -> Self {
        NewTimelineItem::from(ApNote::from(note))
    }
}

impl From<ApNote> for NewTimelineItem {
    fn from(note: ApNote) -> Self {
        NewTimelineItem {
            tag: serde_json::to_value(&note.tag).ok(),
            attributed_to: note.clone().attributed_to.to_string(),
            ap_id: note.clone().id.unwrap(),
            kind: note.kind.to_string(),
            url: note.clone().url,
            published: Some(note.clone().published),
            replies: serde_json::to_value(&note.replies).ok(),
            in_reply_to: note.clone().in_reply_to,
            content: Some(note.clone().content),
            ap_public: {
                let mut public = false;

                let mut addresses = note.to.multiple();
                if let Some(cc) = note.cc {
                    addresses.append(&mut cc.multiple());
                }

                for address in addresses {
                    if !public {
                        public = address.is_public();
                    }
                }
                public
            },
            summary: note.summary,
            ap_sensitive: note.sensitive,
            atom_uri: note.atom_uri,
            in_reply_to_atom_uri: note.in_reply_to_atom_uri,
            conversation: note.conversation,
            content_map: serde_json::to_value(&note.content_map).ok(),
            attachment: serde_json::to_value(&note.attachment).ok(),
            ap_object: None,
            metadata: serde_json::to_value(&note.ephemeral_metadata).ok(),
        }
    }
}

type SynthesizedAnnounce = (ApAnnounce, ApNote);

impl From<SynthesizedAnnounce> for NewTimelineItem {
    fn from((activity, note): SynthesizedAnnounce) -> Self {
        NewTimelineItem {
            tag: Option::from(serde_json::to_value(&note.tag).unwrap_or_default()),
            attributed_to: note.clone().attributed_to.to_string(),
            ap_id: note.clone().id.unwrap(),
            kind: note.kind.to_string(),
            url: note.clone().url,
            published: Some(activity.published),
            replies: Option::from(serde_json::to_value(&note.replies).unwrap_or_default()),
            in_reply_to: note.clone().in_reply_to,
            content: Option::from(note.clone().content),
            ap_public: {
                if let Some(address) = note.to.single() {
                    address.is_public()
                } else {
                    false
                }
            },
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
            metadata: Option::None,
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = timeline)]
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
    pub metadata: Option<Value>,
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = timeline_cc)]
pub struct NewTimelineItemCc {
    pub timeline_id: i32,
    pub ap_id: String,
}

#[derive(Identifiable, Queryable, AsChangeset, Associations, Serialize, Clone, Default, Debug)]
#[diesel(belongs_to(TimelineItem, foreign_key = timeline_id))]
#[diesel(table_name = timeline_cc)]
pub struct TimelineItemCc {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub timeline_id: i32,
    pub ap_id: String,
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = timeline_to)]
pub struct NewTimelineItemTo {
    pub timeline_id: i32,
    pub ap_id: String,
}

#[derive(Identifiable, Queryable, AsChangeset, Associations, Serialize, Clone, Default, Debug)]
#[diesel(belongs_to(TimelineItem, foreign_key = timeline_id))]
#[diesel(table_name = timeline_to)]
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

// this is used in inbox/retrieve to accommodate authenticated calls for
// more detailed timeline data (e.g., to include whether or not I've liked
// a post)
pub type AuthenticatedTimelineItem = (
    TimelineItem,
    Option<Activity>,
    Option<TimelineItemCc>,
    Option<RemoteAnnounce>,
    Option<RemoteLike>,
);

pub async fn get_authenticated_timeline_items(
    conn: &Db,
    limit: i64,
    offset: i64,
    profile: Profile,
) -> Vec<AuthenticatedTimelineItem> {
    conn.run(move |c| {
        let ap_id = get_ap_id_from_username(profile.username.clone());
        let query = timeline::table
            .left_join(
                activities::table.on(activities::target_ap_id
                    .eq(timeline::ap_id.nullable())
                    .and(activities::profile_id.eq(profile.id))
                    .and(activities::revoked.eq(false))),
            )
            .left_join(
                timeline_cc::table.on(timeline_cc::id
                    .eq(timeline::id)
                    .and(timeline_cc::ap_id.eq(ap_id.clone()))),
            )
            .left_join(
                remote_announces::table
                    .on(remote_announces::timeline_id.eq(timeline::id.nullable())),
            )
            .left_join(remote_likes::table.on(remote_likes::object_id.eq(timeline::ap_id)))
            .filter(timeline::ap_public.eq(true))
            .or_filter(timeline_cc::ap_id.eq(ap_id))
            .order(timeline::created_at.desc())
            .limit(limit)
            .offset(offset)
            .into_boxed();

        query.get_results::<AuthenticatedTimelineItem>(c)
    })
    .await
    .unwrap_or(vec![])
}

pub async fn get_public_timeline_items(
    conn: &Db,
    limit: i64,
    offset: i64,
) -> Vec<(TimelineItem, Option<RemoteAnnounce>, Option<RemoteLike>)> {
    conn.run(move |c| {
        let query = timeline::table
            .left_join(
                remote_announces::table
                    .on(remote_announces::timeline_id.eq(timeline::id.nullable())),
            )
            .left_join(remote_likes::table.on(remote_likes::object_id.eq(timeline::ap_id)))
            .filter(timeline::ap_public.eq(true))
            .order(timeline::created_at.desc())
            .limit(limit)
            .offset(offset)
            .into_boxed();

        query.get_results::<(TimelineItem, Option<RemoteAnnounce>, Option<RemoteLike>)>(c)
    })
    .await
    .unwrap_or(vec![])
}

pub async fn get_timeline_item_by_ap_id(conn: &Db, ap_id: String) -> Option<TimelineItem> {
    conn.run(move |c| {
        timeline::table
            .filter(timeline::ap_id.eq(ap_id))
            .first::<TimelineItem>(c)
    })
    .await
    .ok()
}

pub async fn get_timeline_items_by_conversation(
    conn: &Db,
    conversation: String,
    limit: i64,
    offset: i64,
) -> Vec<TimelineItem> {
    conn.run(move |c| {
        let query = timeline::table
            .filter(timeline::conversation.eq(conversation))
            .order(timeline::created_at.asc())
            .limit(limit)
            .offset(offset)
            .into_boxed();

        query.get_results::<TimelineItem>(c)
    })
    .await
    .unwrap_or(vec![])
}

pub async fn delete_timeline_item_by_ap_id(conn: &Db, ap_id: String) -> bool {
    conn.run(move |c| diesel::delete(timeline::table.filter(timeline::ap_id.eq(ap_id))).execute(c))
        .await
        .is_ok()
}
