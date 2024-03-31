use std::fmt::Display;
use std::ops::Add;

use crate::activity_pub::{ApActor, ApAnnounce, ApNote};
use crate::db::Db;
use crate::schema::{timeline, timeline_cc, timeline_to};
use crate::POOL;
use anyhow::Result;
use diesel::prelude::*;
use diesel::{AsChangeset, Insertable};
use serde::{Deserialize, Serialize};

use super::notes::Note;
use super::profiles::Profile;
use super::remote_notes::RemoteNote;
use super::remote_questions::RemoteQuestion;
use super::timeline_hashtags::TimelineHashtag;
use crate::models::activities::{Activity, ActivityCc, ActivityTo};
use crate::models::to_serde;
use crate::routes::inbox::InboxView;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        fn convert_type<T1, T2: std::convert::From<T1>>(kind: T1) -> T2 {
            kind.into()
        }

        pub use crate::models::pg::timeline::NewTimelineItem;
        pub use crate::models::pg::timeline::TimelineType;
        pub use crate::models::pg::timeline::TimelineItem;
        pub use crate::models::pg::timeline::TimelineItemCc;
        pub use crate::models::pg::timeline::TimelineItemTo;
        pub use crate::models::pg::timeline::get_timeline_items;
        pub use crate::models::pg::timeline::create_timeline_item;
        pub use crate::models::pg::timeline::update_timeline_items;
    } else if #[cfg(feature = "sqlite")] {
        fn convert_type<T: Display>(kind: T) -> String {
            kind.to_string().to_lowercase()
        }

        pub use crate::models::sqlite::timeline::NewTimelineItem;
        pub use crate::models::sqlite::timeline::TimelineType;
        pub use crate::models::sqlite::timeline::TimelineItem;
        pub use crate::models::sqlite::timeline::TimelineItemCc;
        pub use crate::models::sqlite::timeline::TimelineItemTo;
        pub use crate::models::sqlite::timeline::get_timeline_items;
        pub use crate::models::sqlite::timeline::create_timeline_item;
        pub use crate::models::sqlite::timeline::update_timeline_items;
    }
}

impl From<RemoteQuestion> for NewTimelineItem {
    fn from(question: RemoteQuestion) -> Self {
        NewTimelineItem {
            ap_public: question.clone().is_public(),
            tag: question.tag,
            attributed_to: question.attributed_to,
            ap_id: question.ap_id,

            ..Default::default()
        }
    }
}

impl From<RemoteNote> for NewTimelineItem {
    fn from(note: RemoteNote) -> Self {
        NewTimelineItem {
            ap_public: note.clone().is_public(),
            tag: note.tag,
            attributed_to: note.attributed_to,
            ap_id: note.ap_id,
            kind: convert_type(note.kind),
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
            end_time: None,
            one_of: None,
            any_of: None,
            voters_count: None,
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
            tag: to_serde(note.tag.clone()),
            attributed_to: note.clone().attributed_to.to_string(),
            ap_id: note.clone().id.unwrap(),
            kind: convert_type(note.clone().kind),
            url: note.clone().url,
            published: Some(note.clone().published),
            replies: to_serde(note.replies.clone()),
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
            content_map: to_serde(note.content_map),
            attachment: to_serde(note.attachment),
            ap_object: None,
            metadata: to_serde(note.ephemeral_metadata),
            end_time: None,
            one_of: None,
            any_of: None,
            voters_count: None,
        }
    }
}

type SynthesizedAnnounce = (Option<ApAnnounce>, ApNote);

impl From<SynthesizedAnnounce> for NewTimelineItem {
    fn from((activity, note): SynthesizedAnnounce) -> Self {
        NewTimelineItem {
            tag: to_serde(note.tag.clone()),
            attributed_to: note.clone().attributed_to.to_string(),
            ap_id: note.clone().id.unwrap(),
            kind: convert_type(note.clone().kind),
            url: note.clone().url,
            published: activity.map_or(Some(note.clone().published), |x| Some(x.published)),
            replies: to_serde(note.replies.clone()),
            in_reply_to: note.clone().in_reply_to,
            content: Some(note.clone().content),
            ap_public: {
                if let Ok(address) = note.to.single() {
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
            content_map: to_serde(note.content_map.unwrap_or_default()),
            attachment: to_serde(note.attachment.unwrap_or_default()),
            ap_object: None,
            metadata: to_serde(note.ephemeral_metadata),
            end_time: None,
            one_of: None,
            any_of: None,
            voters_count: None,
        }
    }
}

#[derive(Serialize, Deserialize, Insertable, AsChangeset, Default, Debug, Clone)]
#[diesel(table_name = timeline_cc)]
pub struct NewTimelineItemCc {
    pub timeline_id: i32,
    pub ap_id: String,
}

#[derive(Serialize, Deserialize, Insertable, AsChangeset, Default, Debug, Clone)]
#[diesel(table_name = timeline_to)]
pub struct NewTimelineItemTo {
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

#[derive(Eq, PartialEq)]
pub enum TimelineView {
    Home(Vec<String>),
    Local,
    Global,
}

impl From<InboxView> for TimelineView {
    fn from(view: InboxView) -> Self {
        match view {
            InboxView::Local => TimelineView::Local,
            InboxView::Global => TimelineView::Global,
            InboxView::Home => TimelineView::Home(vec![]),
        }
    }
}

pub struct TimelineFilters {
    pub view: TimelineView,
    pub hashtags: Vec<String>,
}
// this is used in inbox/retrieve to accommodate authenticated calls for
// more detailed timeline data (e.g., to include whether or not I've liked
// a post - Activity will include CREATE, LIKE, etc from the activities table)
pub type AuthenticatedTimelineItem = (
    TimelineItem,
    Activity,
    ActivityTo,
    Option<ActivityCc>,
    TimelineItemTo,
    Option<TimelineItemCc>,
    Option<TimelineHashtag>,
);

#[derive(Serialize, Clone, Debug, Default)]
pub struct ContextualizedTimelineItem {
    pub item: TimelineItem,
    pub activity: Vec<Activity>,
    pub cc: Vec<TimelineItemCc>,
    // This seems like it should use RemoteActor, but all the supporting functions
    // for populating this Vec output ApActors right now, which doesn't really
    // seem like a problem.
    pub related: Vec<ApActor>,
    pub requester: Option<Profile>,
}

impl Add for ContextualizedTimelineItem {
    type Output = Self;

    fn add(mut self, other: Self) -> Self {
        self.activity.extend(other.activity);
        self.related.extend(other.related);
        self.cc.extend(other.cc);

        Self {
            item: self.item,
            activity: self.activity,
            cc: self.cc,
            related: self.related,
            requester: self.requester.or(other.requester),
        }
    }
}

pub async fn get_timeline_conversation_count(
    conn: Option<&Db>,
    conversation: String,
) -> Result<i64> {
    if let Some(conn) = conn {
        conn.run(move |c| {
            timeline::table
                .filter(timeline::conversation.eq(conversation))
                .count()
                .get_result::<i64>(c)
                .map_err(anyhow::Error::msg)
        })
        .await
    } else {
        let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
        timeline::table
            .filter(timeline::conversation.eq(conversation))
            .count()
            .get_result::<i64>(&mut pool)
            .map_err(anyhow::Error::msg)
    }
}

pub async fn get_timeline_items_by_conversation(
    conn: Option<&Db>,
    conversation: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<TimelineItem>> {
    if let Some(conn) = conn {
        conn.run(move |c| {
            timeline::table
                .filter(timeline::conversation.eq(conversation))
                .order(timeline::created_at.asc())
                .limit(limit)
                .offset(offset)
                .get_results::<TimelineItem>(c)
                .map_err(anyhow::Error::msg)
        })
        .await
    } else {
        let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
        timeline::table
            .filter(timeline::conversation.eq(conversation))
            .order(timeline::created_at.asc())
            .limit(limit)
            .offset(offset)
            .get_results::<TimelineItem>(&mut pool)
            .map_err(anyhow::Error::msg)
    }
}

pub async fn create_timeline_item_to(
    conn: Option<&Db>,
    timeline_item_to: (TimelineItem, String),
) -> bool {
    let timeline_item_to = NewTimelineItemTo::from(timeline_item_to);

    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(timeline_to::table)
                    .values(&timeline_item_to)
                    .on_conflict((timeline_to::timeline_id, timeline_to::ap_id))
                    .do_update()
                    .set(&timeline_item_to)
                    .execute(c)
            })
            .await
            .is_ok(),
        None => POOL.get().map_or(false, |mut pool| {
            diesel::insert_into(timeline_to::table)
                .values(&timeline_item_to)
                .on_conflict((timeline_to::timeline_id, timeline_to::ap_id))
                .do_update()
                .set(&timeline_item_to)
                .execute(&mut pool)
                .is_ok()
        }),
    }
}

pub async fn create_timeline_item_cc(
    conn: Option<&Db>,
    timeline_item_cc: (TimelineItem, String),
) -> bool {
    let timeline_item_cc = NewTimelineItemCc::from(timeline_item_cc);

    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(timeline_cc::table)
                    .values(&timeline_item_cc)
                    .on_conflict((timeline_cc::timeline_id, timeline_cc::ap_id))
                    .do_update()
                    .set(&timeline_item_cc)
                    .execute(c)
            })
            .await
            .is_ok(),
        None => POOL.get().map_or(false, |mut pool| {
            diesel::insert_into(timeline_cc::table)
                .values(&timeline_item_cc)
                .on_conflict((timeline_cc::timeline_id, timeline_cc::ap_id))
                .do_update()
                .set(&timeline_item_cc)
                .execute(&mut pool)
                .is_ok()
        }),
    }
}

#[derive(Debug)]
pub enum TimelineDeleteError {
    ConnectionError,
    DatabaseError(diesel::result::Error),
}

pub async fn delete_timeline_item_by_ap_id(conn: Option<&Db>, ap_id: String) -> Result<usize> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::delete(timeline::table.filter(timeline::ap_id.eq(ap_id))).execute(c)
            })
            .await
            .map_err(anyhow::Error::msg),
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            diesel::delete(timeline::table.filter(timeline::ap_id.eq(ap_id)))
                .execute(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}

pub async fn get_timeline_item_by_ap_id(conn: Option<&Db>, ap_id: String) -> Option<TimelineItem> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                timeline::table
                    .filter(timeline::ap_id.eq(ap_id))
                    .first::<TimelineItem>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            timeline::table
                .filter(timeline::ap_id.eq(ap_id))
                .first::<TimelineItem>(&mut pool)
                .ok()
        }
    }
}
