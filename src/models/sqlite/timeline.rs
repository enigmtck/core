use crate::activity_pub::{ApAnnounce, ApNote};
use crate::db::Db;
use crate::helper::get_ap_id_from_username;
use crate::schema::{
    activities, activities_cc, activities_to, timeline, timeline_cc, timeline_hashtags, timeline_to,
};
use crate::POOL;
use anyhow::Result;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use super::notes::Note;
use super::profiles::Profile;
use super::remote_notes::RemoteNote;
use super::timeline_hashtags::TimelineHashtag;
use crate::models::activities::{Activity, ActivityCc, ActivityTo};
use crate::routes::inbox::InboxView;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone, AsChangeset)]
#[diesel(table_name = timeline)]
pub struct NewTimelineItem {
    pub tag: Option<String>,
    pub attributed_to: String,
    pub ap_id: String,
    pub kind: String,
    pub url: Option<String>,
    pub published: Option<String>,
    pub replies: Option<String>,
    pub in_reply_to: Option<String>,
    pub content: Option<String>,
    pub ap_public: bool,
    pub summary: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub atom_uri: Option<String>,
    pub in_reply_to_atom_uri: Option<String>,
    pub conversation: Option<String>,
    pub content_map: Option<String>,
    pub attachment: Option<String>,
    pub ap_object: Option<String>,
    pub metadata: Option<String>,
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
            tag: serde_json::to_string(&note.tag).ok(),
            attributed_to: note.clone().attributed_to.to_string(),
            ap_id: note.clone().id.unwrap(),
            kind: note.clone().kind.to_string().to_lowercase(),
            url: note.clone().url,
            published: Some(note.clone().published),
            replies: serde_json::to_string(&note.replies).ok(),
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
            content_map: serde_json::to_string(&note.content_map).ok(),
            attachment: serde_json::to_string(&note.attachment).ok(),
            ap_object: None,
            metadata: serde_json::to_string(&note.ephemeral_metadata).ok(),
        }
    }
}

type SynthesizedAnnounce = (Option<ApAnnounce>, ApNote);

impl From<SynthesizedAnnounce> for NewTimelineItem {
    fn from((activity, note): SynthesizedAnnounce) -> Self {
        NewTimelineItem {
            tag: Option::from(serde_json::to_string(&note.tag).unwrap_or_default()),
            attributed_to: note.clone().attributed_to.to_string(),
            ap_id: note.clone().id.unwrap(),
            kind: note.clone().kind.to_string().to_lowercase(),
            url: note.clone().url,
            published: activity.map_or(Some(note.clone().published), |x| Some(x.published)),
            replies: Option::from(serde_json::to_string(&note.replies).unwrap_or_default()),
            in_reply_to: note.clone().in_reply_to,
            content: Option::from(note.clone().content),
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
            content_map: Some(
                serde_json::to_string(&note.content_map.unwrap_or_default()).unwrap_or_default(),
            ),
            attachment: Some(
                serde_json::to_string(&note.attachment.unwrap_or_default()).unwrap_or_default(),
            ),
            ap_object: None,
            metadata: serde_json::to_string(&note.ephemeral_metadata).ok(),
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = timeline)]
pub struct TimelineItem {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub tag: Option<String>,
    pub attributed_to: String,
    pub ap_id: String,
    pub kind: String,
    pub url: Option<String>,
    pub published: Option<String>,
    pub replies: Option<String>,
    pub in_reply_to: Option<String>,
    pub content: Option<String>,
    pub ap_public: bool,
    pub summary: Option<String>,
    pub ap_sensitive: Option<bool>,
    pub atom_uri: Option<String>,
    pub in_reply_to_atom_uri: Option<String>,
    pub conversation: Option<String>,
    pub content_map: Option<String>,
    pub attachment: Option<String>,
    pub ap_object: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Serialize, Deserialize, Insertable, AsChangeset, Default, Debug, Clone)]
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub timeline_id: i32,
    pub ap_id: String,
}

#[derive(Serialize, Deserialize, Insertable, AsChangeset, Default, Debug, Clone)]
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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

pub async fn get_timeline_items(
    conn: &Db,
    limit: i64,
    offset: i64,
    profile: Option<Profile>,
    filters: Option<TimelineFilters>,
) -> Vec<AuthenticatedTimelineItem> {
    conn.run(move |c| {
        let mut query = timeline::table
            .inner_join(
                activities::table.on(activities::target_ap_id
                    .eq(timeline::ap_id.nullable())
                    .and(activities::revoked.eq(false))
                    .and(
                        activities::kind.eq_any(vec!["create".to_string(), "announce".to_string()]),
                    )
                    .and(timeline::in_reply_to.is_null())),
            )
            .inner_join(activities_to::table.on(activities_to::activity_id.eq(activities::id)))
            .left_join(activities_cc::table.on(activities_cc::activity_id.eq(activities::id)))
            .inner_join(timeline_to::table.on(timeline_to::timeline_id.eq(timeline::id)))
            .left_join(timeline_cc::table.on(timeline_cc::timeline_id.eq(timeline::id)))
            .left_join(timeline_hashtags::table.on(timeline_hashtags::timeline_id.eq(timeline::id)))
            .into_boxed();

        if let Some(filters) = filters {
            match filters.view {
                TimelineView::Global => {
                    query = query.filter(timeline::ap_public.eq(true));
                }
                TimelineView::Local => {
                    query = query.filter(activities::profile_id.is_not_null());
                }
                TimelineView::Home(leaders) => {
                    if let Some(profile) = profile {
                        let ap_id = get_ap_id_from_username(profile.username.clone());

                        query = query.filter(
                            timeline_cc::ap_id
                                .eq(ap_id.clone())
                                .or(timeline_cc::ap_id.eq_any(leaders.clone()))
                                .or(timeline_to::ap_id.eq(ap_id.clone()))
                                .or(timeline_to::ap_id.eq_any(leaders.clone()))
                                .or(activities_to::ap_id.eq(ap_id.clone()))
                                .or(activities_to::ap_id.eq_any(leaders.clone()))
                                .or(activities_cc::ap_id.eq(ap_id))
                                .or(activities_cc::ap_id.eq_any(leaders)),
                        );
                    }
                }
            }
            if !filters.hashtags.is_empty() {
                query = query.filter(timeline_hashtags::hashtag.eq_any(filters.hashtags));
            }
        } else {
            query = query.filter(timeline::ap_public.eq(true));
        }

        query
            .order(timeline::created_at.desc())
            .limit(limit)
            .offset(offset)
            .get_results::<AuthenticatedTimelineItem>(c)
    })
    .await
    .unwrap_or(vec![])
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

pub async fn create_timeline_item(
    conn: Option<&Db>,
    timeline_item: NewTimelineItem,
) -> Result<TimelineItem> {
    let timeline_item_clone = timeline_item.clone();
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(timeline::table)
                    .values(&timeline_item_clone)
                    .on_conflict(timeline::ap_id)
                    .do_update()
                    .set(&timeline_item_clone)
                    .execute(c)
            })
            .await
            .map_err(anyhow::Error::msg)?;

            conn.run(move |c| {
                timeline::table
                    .filter(timeline::ap_id.eq(&timeline_item.ap_id))
                    .first::<TimelineItem>(c)
            })
            .await
            .map_err(anyhow::Error::msg)
        }
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            diesel::insert_into(timeline::table)
                .values(&timeline_item_clone)
                .on_conflict(timeline::ap_id)
                .do_update()
                .set(&timeline_item_clone)
                .execute(&mut pool)
                .map_err(anyhow::Error::msg)?;

            timeline::table
                .filter(timeline::ap_id.eq(&timeline_item.ap_id))
                .first::<TimelineItem>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
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

pub async fn update_timeline_items(
    conn: Option<&Db>,
    timeline_item: NewTimelineItem,
) -> Vec<TimelineItem> {
    let timeline_item_clone = timeline_item.clone();
    match conn {
        Some(conn) => {
            let _ = conn
                .run(move |c| {
                    diesel::update(
                        timeline::table.filter(timeline::ap_id.eq(&timeline_item_clone.ap_id)),
                    )
                    .set(timeline::content.eq(&timeline_item_clone.content))
                    .execute(c)
                })
                .await;

            conn.run(move |c| {
                timeline::table
                    .filter(timeline::ap_id.eq(&timeline_item.ap_id))
                    .load::<TimelineItem>(c)
            })
            .await
            .unwrap_or_default()
        }
        None => {
            let mut pool = match POOL.get() {
                Ok(pool) => pool,
                Err(_) => return vec![],
            };

            let _ = diesel::update(
                timeline::table.filter(timeline::ap_id.eq(&timeline_item_clone.ap_id)),
            )
            .set(timeline::content.eq(&timeline_item_clone.content))
            .execute(&mut pool);

            timeline::table
                .filter(timeline::ap_id.eq(&timeline_item.ap_id))
                .load::<TimelineItem>(&mut pool)
                .unwrap_or_default()
        }
    }
}
