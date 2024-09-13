use crate::db::Db;
use crate::helper::get_ap_id_from_username;
use crate::models::pg::activities::ActivityType;
use crate::models::timeline::{AuthenticatedTimelineItem, TimelineFilters, TimelineView};
use crate::schema::{
    activities, activities_cc, activities_to, timeline, timeline_cc, timeline_hashtags, timeline_to,
};
use crate::POOL;
use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::Nullable;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

use super::notes::NoteType;
use super::profiles::Profile;
use super::remote_questions::QuestionType;
use crate::activity_pub::ApNoteType;

#[derive(
    diesel_derive_enum::DbEnum, Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq,
)]
#[ExistingTypePath = "crate::schema::sql_types::TimelineType"]
pub enum TimelineType {
    #[default]
    Note,
    Question,
}

impl fmt::Display for TimelineType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<NoteType> for TimelineType {
    fn from(_kind: NoteType) -> Self {
        TimelineType::Note
    }
}

impl From<ApNoteType> for TimelineType {
    fn from(_kind: ApNoteType) -> Self {
        TimelineType::Note
    }
}

impl From<QuestionType> for TimelineType {
    fn from(_kind: QuestionType) -> Self {
        TimelineType::Question
    }
}

#[derive(
    Serialize, Deserialize, Insertable, Default, Debug, Clone, AsChangeset, QueryableByName,
)]
#[diesel(table_name = timeline)]
pub struct NewTimelineItem {
    pub tag: Option<Value>,
    pub attributed_to: String,
    pub ap_id: String,
    pub kind: TimelineType,
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
    pub end_time: Option<DateTime<Utc>>,
    pub one_of: Option<Value>,
    pub any_of: Option<Value>,
    pub voters_count: Option<i32>,
}

#[derive(
    Identifiable,
    Queryable,
    AsChangeset,
    Serialize,
    Deserialize,
    Clone,
    Default,
    Debug,
    QueryableByName,
)]
#[diesel(table_name = timeline)]
pub struct TimelineItem {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tag: Option<Value>,
    pub attributed_to: String,
    pub ap_id: String,
    pub kind: TimelineType,
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
    pub end_time: Option<DateTime<Utc>>,
    pub one_of: Option<Value>,
    pub any_of: Option<Value>,
    pub voters_count: Option<i32>,
}

#[derive(
    Identifiable,
    Queryable,
    AsChangeset,
    Associations,
    Serialize,
    Deserialize,
    Clone,
    Default,
    Debug,
    QueryableByName,
)]
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

#[derive(
    Identifiable,
    Queryable,
    AsChangeset,
    Associations,
    Serialize,
    Deserialize,
    Clone,
    Default,
    Debug,
    QueryableByName,
)]
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
                        activities::kind.eq_any(vec![ActivityType::Create, ActivityType::Announce]),
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

pub async fn create_timeline_item(
    conn: Option<&Db>,
    timeline_item: NewTimelineItem,
) -> Result<TimelineItem> {
    if let Some(conn) = conn {
        conn.run(move |c| {
            diesel::insert_into(timeline::table)
                .values(&timeline_item)
                .on_conflict(timeline::ap_id)
                .do_update()
                .set(&timeline_item)
                .get_result::<TimelineItem>(c)
                .map_err(anyhow::Error::msg)
        })
        .await
    } else {
        let mut pool = POOL.get().map_err(anyhow::Error::msg)?;

        diesel::insert_into(timeline::table)
            .values(&timeline_item)
            .on_conflict(timeline::ap_id)
            .do_update()
            .set(&timeline_item)
            .get_result::<TimelineItem>(&mut pool)
            .map_err(anyhow::Error::msg)
    }
}

pub async fn update_timeline_items(
    conn: Option<&Db>,
    timeline_item: NewTimelineItem,
) -> Vec<TimelineItem> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::update(timeline::table.filter(timeline::ap_id.eq(timeline_item.ap_id)))
                    .set(timeline::content.eq(timeline_item.content))
                    .get_results::<TimelineItem>(c)
            })
            .await
            .unwrap_or(vec![]),
        None => POOL.get().map_or(vec![], |mut pool| {
            diesel::update(timeline::table.filter(timeline::ap_id.eq(timeline_item.ap_id)))
                .set(timeline::content.eq(timeline_item.content))
                .get_results::<TimelineItem>(&mut pool)
                .unwrap_or(vec![])
        }),
    }
}
