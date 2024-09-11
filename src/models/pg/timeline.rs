use crate::db::Db;
use crate::helper::get_ap_id_from_username;
use crate::models::pg::activities::ActivityType;
use crate::models::timeline::{AuthenticatedTimelineItem, TimelineFilters, TimelineView};
use crate::schema::sql_types::{ActivityType as SqlActivityType, TimelineType as SqlTimelineType};
use crate::schema::{
    activities, activities_cc, activities_to, timeline, timeline_cc, timeline_hashtags, timeline_to,
};
use crate::POOL;
use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::{Bool, Integer, Jsonb, Nullable, Text, Timestamptz};
use diesel::{sql_query, AsChangeset, Identifiable, Insertable, Queryable};
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

impl From<JoinedTimelineItem> for TimelineItem {
    fn from(item: JoinedTimelineItem) -> Self {
        let id = item.timeline_id;
        let created_at = item.timeline_created_at;
        let updated_at = item.timeline_updated_at;
        let tag = item.timeline_tag;
        let attributed_to = item.timeline_attributed_to;
        let ap_id = item.timeline_ap_id;
        let kind = item.timeline_kind;
        let url = item.timeline_url;
        let published = item.timeline_published;
        let replies = item.timeline_replies;
        let in_reply_to = item.timeline_in_reply_to;
        let content = item.timeline_content;
        let ap_public = item.timeline_ap_public;
        let summary = item.timeline_summary;
        let ap_sensitive = item.timeline_ap_sensitive;
        let atom_uri = item.timeline_atom_uri;
        let in_reply_to_atom_uri = item.timeline_in_reply_to_atom_uri;
        let conversation = item.timeline_conversation;
        let content_map = item.timeline_content_map;
        let attachment = item.timeline_attachment;
        let ap_object = item.timeline_ap_object;
        let metadata = item.timeline_metadata;
        let end_time = item.timeline_end_time;
        let one_of = item.timeline_one_of;
        let any_of = item.timeline_any_of;
        let voters_count = item.timeline_voters_count;

        TimelineItem {
            id,
            created_at,
            updated_at,
            tag,
            attributed_to,
            ap_id,
            kind,
            url,
            published,
            replies,
            in_reply_to,
            content,
            ap_public,
            summary,
            ap_sensitive,
            atom_uri,
            in_reply_to_atom_uri,
            conversation,
            content_map,
            attachment,
            ap_object,
            metadata,
            end_time,
            one_of,
            any_of,
            voters_count,
        }
    }
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

impl TryFrom<JoinedTimelineItem> for TimelineItemCc {
    type Error = anyhow::Error;

    fn try_from(item: JoinedTimelineItem) -> Result<Self, Self::Error> {
        let id = item
            .timeline_cc_id
            .ok_or_else(|| anyhow::Error::msg("missing id"))?;
        let created_at = item
            .timeline_cc_created_at
            .ok_or_else(|| anyhow::Error::msg("missing created_at"))?;
        let updated_at = item
            .timeline_cc_updated_at
            .ok_or_else(|| anyhow::Error::msg("missing updated_at"))?;
        let timeline_id = item
            .timeline_cc_timeline_id
            .ok_or_else(|| anyhow::Error::msg("missing timeline_id"))?;
        let ap_id = item
            .timeline_cc_ap_id
            .ok_or_else(|| anyhow::Error::msg("missing ap_id"))?;

        Ok(TimelineItemCc {
            id,
            created_at,
            updated_at,
            timeline_id,
            ap_id,
        })
    }
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

impl From<JoinedTimelineItem> for TimelineItemTo {
    fn from(item: JoinedTimelineItem) -> Self {
        let id = item.timeline_to_id;
        let created_at = item.timeline_to_created_at;
        let updated_at = item.timeline_to_updated_at;
        let timeline_id = item.timeline_to_timeline_id;
        let ap_id = item.timeline_to_ap_id;

        TimelineItemTo {
            id,
            created_at,
            updated_at,
            timeline_id,
            ap_id,
        }
    }
}

#[derive(Queryable, Serialize, Deserialize, Clone, Default, Debug, QueryableByName)]
pub struct JoinedTimelineItem {
    #[sql_type = "Integer"]
    pub timeline_id: i32,

    #[sql_type = "Timestamptz"]
    pub timeline_created_at: DateTime<Utc>,

    #[sql_type = "Timestamptz"]
    pub timeline_updated_at: DateTime<Utc>,

    #[sql_type = "Nullable<Jsonb>"]
    pub timeline_tag: Option<Value>,

    #[sql_type = "Text"]
    pub timeline_attributed_to: String,

    #[sql_type = "Text"]
    pub timeline_ap_id: String,

    #[sql_type = "SqlTimelineType"]
    pub timeline_kind: TimelineType,

    #[sql_type = "Nullable<Text>"]
    pub timeline_url: Option<String>,

    #[sql_type = "Nullable<Text>"]
    pub timeline_published: Option<String>,

    #[sql_type = "Nullable<Jsonb>"]
    pub timeline_replies: Option<Value>,

    #[sql_type = "Nullable<Text>"]
    pub timeline_in_reply_to: Option<String>,

    #[sql_type = "Nullable<Text>"]
    pub timeline_content: Option<String>,

    #[sql_type = "Bool"]
    pub timeline_ap_public: bool,

    #[sql_type = "Nullable<Text>"]
    pub timeline_summary: Option<String>,

    #[sql_type = "Nullable<Bool>"]
    pub timeline_ap_sensitive: Option<bool>,

    #[sql_type = "Nullable<Text>"]
    pub timeline_atom_uri: Option<String>,

    #[sql_type = "Nullable<Text>"]
    pub timeline_in_reply_to_atom_uri: Option<String>,

    #[sql_type = "Nullable<Text>"]
    pub timeline_conversation: Option<String>,

    #[sql_type = "Nullable<Jsonb>"]
    pub timeline_content_map: Option<Value>,

    #[sql_type = "Nullable<Jsonb>"]
    pub timeline_attachment: Option<Value>,

    #[sql_type = "Nullable<Jsonb>"]
    pub timeline_ap_object: Option<Value>,

    #[sql_type = "Nullable<Jsonb>"]
    pub timeline_metadata: Option<Value>,

    #[sql_type = "Nullable<Timestamptz>"]
    pub timeline_end_time: Option<DateTime<Utc>>,

    #[sql_type = "Nullable<Jsonb>"]
    pub timeline_one_of: Option<Value>,

    #[sql_type = "Nullable<Jsonb>"]
    pub timeline_any_of: Option<Value>,

    #[sql_type = "Nullable<Integer>"]
    pub timeline_voters_count: Option<i32>,

    #[sql_type = "Integer"]
    pub activity_id: i32,

    #[sql_type = "Timestamptz"]
    pub activity_created_at: DateTime<Utc>,

    #[sql_type = "Timestamptz"]
    pub activity_updated_at: DateTime<Utc>,

    #[sql_type = "Nullable<Integer>"]
    pub activity_profile_id: Option<i32>,

    #[sql_type = "SqlActivityType"]
    pub activity_kind: ActivityType,

    #[sql_type = "Text"]
    pub activity_uuid: String,

    #[sql_type = "Text"]
    pub activity_actor: String,

    #[sql_type = "Nullable<Jsonb>"]
    pub activity_ap_to: Option<Value>,

    #[sql_type = "Nullable<Jsonb>"]
    pub activity_cc: Option<Value>,

    #[sql_type = "Nullable<Integer>"]
    pub activity_target_note_id: Option<i32>,

    #[sql_type = "Nullable<Integer>"]
    pub activity_target_remote_note_id: Option<i32>,

    #[sql_type = "Nullable<Integer>"]
    pub activity_target_profile_id: Option<i32>,

    #[sql_type = "Nullable<Integer>"]
    pub activity_target_activity_id: Option<i32>,

    #[sql_type = "Nullable<Text>"]
    pub activity_target_ap_id: Option<String>,

    #[sql_type = "Nullable<Integer>"]
    pub activity_target_remote_actor_id: Option<i32>,

    #[sql_type = "Bool"]
    pub activity_revoked: bool,

    #[sql_type = "Nullable<Text>"]
    pub activity_ap_id: Option<String>,

    #[sql_type = "Nullable<Integer>"]
    pub activity_target_remote_question_id: Option<i32>,

    #[sql_type = "Integer"]
    pub activity_to_id: i32,

    #[sql_type = "Timestamptz"]
    pub activity_to_created_at: DateTime<Utc>,

    #[sql_type = "Timestamptz"]
    pub activity_to_updated_at: DateTime<Utc>,

    #[sql_type = "Integer"]
    pub activity_to_activity_id: i32,

    #[sql_type = "Text"]
    pub activity_to_ap_id: String,

    #[sql_type = "Nullable<Integer>"]
    pub activity_cc_id: Option<i32>,

    #[sql_type = "Nullable<Timestamptz>"]
    pub activity_cc_created_at: Option<DateTime<Utc>>,

    #[sql_type = "Nullable<Timestamptz>"]
    pub activity_cc_updated_at: Option<DateTime<Utc>>,

    #[sql_type = "Nullable<Integer>"]
    pub activity_cc_activity_id: Option<i32>,

    #[sql_type = "Nullable<Text>"]
    pub activity_cc_ap_id: Option<String>,

    #[sql_type = "Integer"]
    pub timeline_to_id: i32,

    #[sql_type = "Timestamptz"]
    pub timeline_to_created_at: DateTime<Utc>,

    #[sql_type = "Timestamptz"]
    pub timeline_to_updated_at: DateTime<Utc>,

    #[sql_type = "Integer"]
    pub timeline_to_timeline_id: i32,

    #[sql_type = "Text"]
    pub timeline_to_ap_id: String,

    #[sql_type = "Nullable<Integer>"]
    pub timeline_cc_id: Option<i32>,

    #[sql_type = "Nullable<Timestamptz>"]
    pub timeline_cc_created_at: Option<DateTime<Utc>>,

    #[sql_type = "Nullable<Timestamptz>"]
    pub timeline_cc_updated_at: Option<DateTime<Utc>>,

    #[sql_type = "Nullable<Integer>"]
    pub timeline_cc_timeline_id: Option<i32>,

    #[sql_type = "Nullable<Text>"]
    pub timeline_cc_ap_id: Option<String>,

    #[sql_type = "Nullable<Integer>"]
    pub timeline_hashtag_id: Option<i32>,

    #[sql_type = "Nullable<Timestamptz>"]
    pub timeline_hashtag_created_at: Option<DateTime<Utc>>,

    #[sql_type = "Nullable<Timestamptz>"]
    pub timeline_hashtag_updated_at: Option<DateTime<Utc>>,

    #[sql_type = "Nullable<Text>"]
    pub timeline_hashtag_hashtag: Option<String>,

    #[sql_type = "Nullable<Integer>"]
    pub timeline_hashtag_timeline_id: Option<i32>,
}

pub async fn get_timeline_items_raw(
    conn: &Db,
    limit: i64,
    min: Option<i64>,
    max: Option<i64>,
    profile: Option<Profile>,
    filters: Option<TimelineFilters>,
) -> Vec<AuthenticatedTimelineItem> {
    conn.run(move |c| {
        // Diesel doesn't support composite queries, so this is a way to specify
        // a limit on the main table that accommodates the duplicate records generated
        // by table joins
        let mut query = "SELECT t.id AS timeline_id,\
             t.created_at AS timeline_created_at,\
             t.updated_at AS timeline_updated_at,\
             t.tag AS timeline_tag,\
             t.attributed_to AS timeline_attributed_to,\
             t.ap_id AS timeline_ap_id,\
             t.kind AS timeline_kind,\
             t.url AS timeline_url,\
             t.published AS timeline_published,\
             t.replies AS timeline_replies,\
             t.in_reply_to AS timeline_in_reply_to,\
             t.content AS timeline_content,\
             t.ap_public AS timeline_ap_public,\
             t.summary AS timeline_summary,\
             t.ap_sensitive AS timeline_ap_sensitive,\
             t.atom_uri AS timeline_atom_uri,\
             t.in_reply_to_atom_uri AS timeline_in_reply_to_atom_uri,\
             t.conversation AS timeline_conversation,\
             t.content_map AS timeline_content_map,\
             t.attachment AS timeline_attachment,\
             t.ap_object AS timeline_ap_object,\
             t.metadata AS timeline_metadata,\
             t.end_time AS timeline_end_time,\
             t.one_of AS timeline_one_of,\
             t.any_of AS timeline_any_of,\
             t.voters_count AS timeline_voters_count,\
             a.id AS activity_id,\
             a.created_at AS activity_created_at,\
             a.updated_at AS activity_updated_at,\
             a.profile_id AS activity_profile_id,\
             a.kind AS activity_kind,\
             a.uuid AS activity_uuid,\
             a.actor AS activity_actor,\
             a.ap_to AS activity_ap_to,\
             a.cc AS activity_cc,\
             a.target_note_id AS activity_target_note_id,\
             a.target_remote_note_id AS activity_target_remote_note_id,\
             a.target_profile_id AS activity_target_profile_id,\
             a.target_activity_id AS activity_target_activity_id,\
             a.target_ap_id AS activity_target_ap_id,\
             a.target_remote_actor_id AS activity_target_remote_actor_id,\
             a.revoked AS activity_revoked,\
             a.ap_id AS activity_ap_id,\
             a.target_remote_question_id AS activity_target_remote_question_id,\
             at.id AS activity_to_id,\
             at.created_at AS activity_to_created_at,\
             at.updated_at AS activity_to_updated_at,\
             at.activity_id AS activity_to_activity_id,\
             at.ap_id AS activity_to_ap_id,\
             ac.id AS activity_cc_id,\
             ac.created_at AS activity_cc_created_at,\
             ac.updated_at AS activity_cc_updated_at,\
             ac.activity_id AS activity_cc_activity_id,\
             ac.ap_id AS activity_cc_ap_id,\
             tt.id AS timeline_to_id,\
             tt.created_at AS timeline_to_created_at,\
             tt.updated_at AS timeline_to_updated_at,\
             tt.timeline_id AS timeline_to_timeline_id,\
             tt.ap_id AS timeline_to_ap_id,\
             tc.id AS timeline_cc_id,\
             tc.created_at AS timeline_cc_created_at,\
             tc.updated_at AS timeline_cc_updated_at,\
             tc.timeline_id AS timeline_cc_timeline_id,\
             tc.ap_id AS timeline_cc_ap_id,\
             th.id AS timeline_hashtag_id,\
             th.created_at AS timeline_hashtag_created_at,\
             th.updated_at AS timeline_hashtag_updated_at,\
             th.hashtag AS timeline_hashtag_hashtag,\
             th.timeline_id AS timeline_hashtag_timeline_id \
             FROM (SELECT * from timeline WHERE TRUE"
            .to_string();

        // Add date filtering to the subquery
        if let Some(min) = min {
            let date: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_micros(min).unwrap(),
                Utc,
            );

            query.push_str(&format!(
                " AND created_at > '{}' ORDER BY created_at ASC",
                date
            ));
        } else if let Some(max) = max {
            let date: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_micros(max).unwrap(),
                Utc,
            );

            query.push_str(&format!(
                " AND created_at < '{}' ORDER BY created_at DESC",
                date
            ));
        } else {
            query.push_str(" ORDER BY created_at DESC");
        }

        query.push_str(&format!(" LIMIT {}) AS t ", limit));
        query.push_str(
            "INNER JOIN activities a ON (t.ap_id = a.target_ap_id and a.revoked = 'f' \
             AND (a.kind = 'create' OR a.kind = 'announce')) \
             INNER JOIN activities_to at ON (a.id = at.activity_id) \
             LEFT JOIN activities_cc ac ON (a.id = ac.activity_id) \
             INNER JOIN timeline_to tt ON (t.id = tt.timeline_id) \
             LEFT JOIN timeline_cc tc ON (t.id = tc.timeline_id) \
             LEFT JOIN timeline_hashtags th ON (t.id = th.timeline_id) \
             WHERE TRUE",
        );

        // Add filters based on the provided options
        if let Some(filters) = filters {
            match filters.view {
                TimelineView::Global => {
                    query.push_str(" AND t.ap_public = TRUE");
                }
                TimelineView::Local => {
                    query.push_str(" AND a.profile_id IS NOT NULL");
                }
                TimelineView::Home(leaders) => {
                    if let Some(profile) = profile {
                        let ap_id = get_ap_id_from_username(profile.username.clone());
                        let leaders_list = leaders.join("','");
                        query.push_str(&format!(
                            " AND (tc.ap_id = '{}' OR tc.ap_id IN ('{}') OR \
                             tt.ap_id = '{}' OR tt.ap_id IN ('{}') OR \
                             at.ap_id = '{}' OR at.ap_id IN ('{}') OR \
                             ac.ap_id = '{}' OR ac.ap_id IN ('{}'))",
                            ap_id,
                            leaders_list,
                            ap_id,
                            leaders_list,
                            ap_id,
                            leaders_list,
                            ap_id,
                            leaders_list
                        ));
                    }
                }
            }

            if !filters.hashtags.is_empty() {
                let hashtags_list = filters.hashtags.join("','");
                query.push_str(&format!(" AND th.hashtag IN ('{}')", hashtags_list));
            }
        } else {
            query.push_str(" AND t.ap_public = TRUE");
        }

        log::debug!("QUERY\n{query:#?}");

        sql_query(query)
            .load(c)
            .expect("bad sql query")
            .iter()
            .map(AuthenticatedTimelineItem::from)
            .collect()
    })
    .await
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
