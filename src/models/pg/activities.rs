use crate::activity_pub::PUBLIC_COLLECTION;
use crate::db::Db;
use crate::helper::get_ap_id_from_username;
use crate::models::timeline::{TimelineFilters, TimelineView};
use crate::schema::{
    activities, activities_cc, activities_to, notes, profiles, remote_actors, remote_note_hashtags,
    remote_notes, remote_questions,
};
use crate::{MaybeMultiple, POOL};
use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::sql_types::Nullable;
use diesel::{prelude::*, sql_query};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::coalesced_activity::CoalescedActivity;
use super::profiles::Profile;
use super::timeline::JoinedTimelineItem;
use crate::models::activities::{
    transform_records_to_extended_activities, ExtendedActivity, ExtendedActivityRecord,
    NewActivityCc, NewActivityTo,
};

#[derive(
    diesel_derive_enum::DbEnum, Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq,
)]
#[ExistingTypePath = "crate::schema::sql_types::ActivityType"]
pub enum ActivityType {
    #[default]
    Create,
    Delete,
    Update,
    Announce,
    Like,
    Undo,
    Follow,
    Accept,
    Block,
    Add,
    Remove,
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = activities)]
pub struct NewActivity {
    pub kind: ActivityType,
    pub uuid: String,
    pub actor: String,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub profile_id: Option<i32>,
    pub target_note_id: Option<i32>,
    pub target_remote_note_id: Option<i32>,
    pub target_profile_id: Option<i32>,
    pub target_activity_id: Option<i32>,
    pub target_ap_id: Option<String>,
    pub target_remote_actor_id: Option<i32>,
    pub revoked: bool,
    pub ap_id: Option<String>,
    pub target_remote_question_id: Option<i32>,
    pub reply: bool,
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
    PartialEq,
    Eq,
    QueryableByName,
)]
#[diesel(table_name = activities)]
pub struct Activity {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile_id: Option<i32>,
    pub kind: ActivityType,
    pub uuid: String,
    pub actor: String,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub target_note_id: Option<i32>,
    pub target_remote_note_id: Option<i32>,
    pub target_profile_id: Option<i32>,
    pub target_activity_id: Option<i32>,
    pub target_ap_id: Option<String>,
    pub target_remote_actor_id: Option<i32>,
    pub revoked: bool,
    pub ap_id: Option<String>,
    pub target_remote_question_id: Option<i32>,
    pub reply: bool,
}

impl From<JoinedTimelineItem> for Activity {
    fn from(item: JoinedTimelineItem) -> Self {
        let id = item.activity_id;
        let created_at = item.activity_created_at;
        let updated_at = item.activity_updated_at;
        let profile_id = item.activity_profile_id;
        let kind = item.activity_kind;
        let uuid = item.activity_uuid;
        let actor = item.activity_actor;
        let ap_to = item.activity_ap_to;
        let cc = item.activity_cc;
        let target_note_id = item.activity_target_note_id;
        let target_remote_note_id = item.activity_target_remote_note_id;
        let target_profile_id = item.activity_target_profile_id;
        let target_activity_id = item.activity_target_activity_id;
        let target_ap_id = item.activity_target_ap_id;
        let target_remote_actor_id = item.activity_target_remote_actor_id;
        let revoked = item.activity_revoked;
        let ap_id = item.activity_ap_id;
        let target_remote_question_id = item.activity_target_remote_question_id;
        let reply = false;

        Activity {
            id,
            created_at,
            updated_at,
            profile_id,
            kind,
            uuid,
            actor,
            ap_to,
            cc,
            target_note_id,
            target_remote_note_id,
            target_profile_id,
            target_activity_id,
            target_ap_id,
            target_remote_actor_id,
            revoked,
            ap_id,
            target_remote_question_id,
            reply,
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
#[diesel(belongs_to(Activity, foreign_key = activity_id))]
#[diesel(table_name = activities_cc)]
pub struct ActivityCc {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub activity_id: i32,
    pub ap_id: String,
}

impl TryFrom<JoinedTimelineItem> for ActivityCc {
    type Error = anyhow::Error;

    fn try_from(item: JoinedTimelineItem) -> Result<Self, Self::Error> {
        let id = item
            .activity_cc_id
            .ok_or_else(|| anyhow::Error::msg("missing id"))?;
        let created_at = item
            .activity_cc_created_at
            .ok_or_else(|| anyhow::Error::msg("missing created_at"))?;
        let updated_at = item
            .activity_cc_updated_at
            .ok_or_else(|| anyhow::Error::msg("missing updated_at"))?;
        let activity_id = item
            .activity_cc_activity_id
            .ok_or_else(|| anyhow::Error::msg("missing activity_id"))?;
        let ap_id = item
            .activity_cc_ap_id
            .ok_or_else(|| anyhow::Error::msg("missing ap_id"))?;

        Ok(ActivityCc {
            id,
            created_at,
            updated_at,
            activity_id,
            ap_id,
        })
    }
}

fn parameter_generator() -> impl FnMut() -> String {
    let mut counter = 1;
    move || {
        let param = format!("${}", counter);
        counter += 1;
        param
    }
}

pub async fn get_activities_coalesced(
    conn: &Db,
    limit: i32,
    min: Option<i64>,
    max: Option<i64>,
    profile: Option<Profile>,
    filters: Option<TimelineFilters>,
) -> Vec<CoalescedActivity> {
    let mut to: Vec<String> = vec![];
    let mut hashtags: Vec<String> = vec![];
    let mut date = Utc::now();

    log::debug!("IN COALESCED");
    let mut param_gen = parameter_generator();

    conn.run(move |c| {
        let mut query = format!(
            "SELECT a.*,\
       COALESCE(n.uuid, NULL) AS object_uuid,\
       COALESCE(r.kind::text, n.kind::text, q.kind::text, NULL) AS object_type,\
       COALESCE(r.published, n.created_at::text, q.published::text, NULL) AS object_published,\
       COALESCE(r.ap_id, n.ap_id, q.ap_id, NULL) AS object_id,\
       COALESCE(r.url, q.url, NULL) AS object_url,\
       COALESCE(r.ap_to, n.ap_to, q.ap_to, NULL) AS object_to,\
       COALESCE(r.cc, n.cc, q.cc, NULL) AS object_cc,\
       COALESCE(r.tag, n.tag, q.tag, NULL) AS object_tag,\
       COALESCE(r.attributed_to, n.attributed_to, q.attributed_to, NULL) AS object_attributed_to,\
       COALESCE(r.in_reply_to, n.in_reply_to, q.in_reply_to, NULL) AS object_in_reply_to,\
       COALESCE(r.content, n.content, q.content, NULL) AS object_content,\
       COALESCE(r.conversation, n.conversation, q.conversation, NULL) AS object_conversation,\
       COALESCE(r.attachment, n.attachment, q.attachment, NULL) AS object_attachment,\
       COALESCE(r.summary, q.summary, NULL) AS object_summary,\
       COALESCE(q.end_time, NULL) AS object_end_time,\
       COALESCE(q.one_of, NULL) AS object_one_of,\
       COALESCE(q.any_of, NULL) AS object_any_of,\
       COALESCE(q.voters_count, NULL) AS object_voters_count,\
       COALESCE(r.ap_sensitive, q.ap_sensitive) AS object_sensitive \
  FROM activities a \
       LEFT JOIN notes n ON (n.id = a.target_note_id) \
       LEFT JOIN remote_notes r ON (r.id = a.target_remote_note_id) \
       LEFT JOIN remote_questions q ON (q.id = a.target_remote_question_id) \
       LEFT JOIN remote_note_hashtags h ON (r.id = h.remote_note_id) \
 WHERE a.kind IN ('announce','create') \
   AND NOT a.reply \
   AND (a.target_note_id IS NOT NULL OR a.target_remote_note_id IS NOT NULL OR a.target_remote_question_id IS NOT NULL) \
   AND a.ap_to ?| {}",
            param_gen()
        );

        // Add date filtering to the subquery
        if let Some(min) = min {
            date = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_micros(min).unwrap(),
                Utc,
            );

            query.push_str(&format!(" AND a.created_at > {}", param_gen()));
        } else if let Some(max) = max {
            date = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_micros(max).unwrap(),
                Utc,
            );

            query.push_str(&format!(" AND a.created_at < {}", param_gen()));
        }

        // Add filters based on the provided options
        if let Some(filters) = filters {
            hashtags.extend(filters.hashtags);

            match filters.view {
                TimelineView::Global => {
                    to.extend((*PUBLIC_COLLECTION).clone());
                }
                TimelineView::Local => {
                    to.extend((*PUBLIC_COLLECTION).clone());
                }
                TimelineView::Home(leaders) => {
                    if let Some(profile) = profile {
                        let ap_id = get_ap_id_from_username(profile.username.clone());
                        to.extend(vec![ap_id]);
                        to.extend(leaders);
                    }
                }
            }

            if !hashtags.is_empty() {
                query.push_str(&format!(" AND to_jsonb(h.hashtag) ?| {}", param_gen()));
            }
        }

        query.push_str(&format!(
            " ORDER BY a.created_at DESC LIMIT {}",
            param_gen()
        ));

        log::debug!("COALESCED QUERY\n{query:#?}");

        let mut query = sql_query(query).into_boxed();
        query = query.bind::<diesel::sql_types::Array<diesel::sql_types::Text>, _>(&to);

        if min.is_some() || max.is_some() {
            query = query.bind::<diesel::sql_types::Timestamptz, _>(&date);
        }

        if !hashtags.is_empty() {
            query = query.bind::<diesel::sql_types::Array<diesel::sql_types::Text>, _>(&hashtags);
        }
        query = query.bind::<diesel::sql_types::Integer, _>(&limit);

        query.load::<CoalescedActivity>(c).expect("bad sql query")
    })
    .await
}

pub async fn create_activity_cc(conn: Option<&Db>, activity_cc: NewActivityCc) -> bool {
    log::debug!("INSERTING ACTIVITY_CC: {activity_cc:#?}");

    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(activities_cc::table)
                    .values(&activity_cc)
                    .get_result::<ActivityCc>(c)
            })
            .await
            .is_ok(),
        None => POOL.get().map_or(false, |mut pool| {
            diesel::insert_into(activities_cc::table)
                .values(&activity_cc)
                .get_result::<ActivityCc>(&mut pool)
                .is_ok()
        }),
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
#[diesel(belongs_to(Activity, foreign_key = activity_id))]
#[diesel(table_name = activities_to)]
pub struct ActivityTo {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub activity_id: i32,
    pub ap_id: String,
}

impl From<JoinedTimelineItem> for ActivityTo {
    fn from(item: JoinedTimelineItem) -> Self {
        let id = item.activity_to_id;
        let created_at = item.activity_to_created_at;
        let updated_at = item.activity_to_updated_at;
        let activity_id = item.activity_to_activity_id;
        let ap_id = item.activity_to_ap_id;

        ActivityTo {
            id,
            created_at,
            updated_at,
            activity_id,
            ap_id,
        }
    }
}

pub async fn create_activity_to(
    conn: Option<&Db>,
    activity_to: NewActivityTo,
) -> Result<ActivityTo> {
    log::debug!("INSERTING ACTIVITY_TO: {activity_to:#?}");

    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(activities_to::table)
                    .values(&activity_to)
                    .get_result::<ActivityTo>(c)
                    .map_err(anyhow::Error::msg)
            })
            .await
        }
        None => POOL.get().map_or(
            Err(anyhow!("failed to retrieve database connection")),
            |mut pool| {
                diesel::insert_into(activities_to::table)
                    .values(&activity_to)
                    .get_result::<ActivityTo>(&mut pool)
                    .map_err(anyhow::Error::msg)
            },
        ),
    }
}

pub async fn create_activity(conn: Option<&Db>, activity: NewActivity) -> Result<Activity> {
    let activity = match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(activities::table)
                    .values(&activity)
                    .get_result::<Activity>(c)
            })
            .await?
        }
        None => {
            let mut pool = POOL.get()?;
            diesel::insert_into(activities::table)
                .values(&activity)
                .get_result::<Activity>(&mut pool)?
        }
    };

    if let Some(ap_to) = activity.clone().ap_to {
        let to: MaybeMultiple<String> = serde_json::from_value(ap_to).map_err(|e| anyhow!(e))?;

        for to in to.multiple() {
            let _ = create_activity_to(conn, (activity.clone(), to).into()).await;
        }
    }

    if let Some(cc) = activity.clone().cc {
        let cc: MaybeMultiple<String> = serde_json::from_value(cc).map_err(|e| anyhow!(e))?;

        for cc in cc.multiple() {
            let _ = create_activity_cc(conn, (activity.clone(), cc).into()).await;
        }
    }

    Ok(activity)
}

pub async fn get_outbox_activities_by_profile_id(
    conn: &Db,
    profile_id: i32,
    min: Option<i64>,
    max: Option<i64>,
    limit: Option<u8>,
) -> Vec<ExtendedActivity> {
    conn.run(move |c| {
        let mut query = activities::table
            .filter(activities::revoked.eq(false))
            .filter(activities::profile_id.eq(profile_id))
            .filter(
                activities::kind
                    .eq(ActivityType::Create)
                    .or(activities::kind.eq(ActivityType::Announce)),
            )
            .left_join(notes::table.on(activities::target_note_id.eq(notes::id.nullable())))
            .left_join(
                remote_notes::table
                    .on(activities::target_remote_note_id.eq(remote_notes::id.nullable())),
            )
            .left_join(
                profiles::table.on(activities::target_profile_id.eq(profiles::id.nullable())),
            )
            .left_join(
                remote_actors::table
                    .on(activities::target_remote_actor_id.eq(remote_actors::id.nullable())),
            )
            .left_join(
                remote_questions::table
                    .on(activities::target_remote_question_id.eq(remote_questions::id.nullable())),
            )
            .left_join(
                remote_note_hashtags::table.on(remote_note_hashtags::remote_note_id
                    .nullable()
                    .eq(remote_notes::id.nullable())),
            )
            .into_boxed();

        if let Some(limit) = limit {
            query = query.limit(limit.into());
        }

        if let Some(min) = min {
            let date: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_micros(min).unwrap(),
                Utc,
            );

            log::debug!("MINIMUM {date:#?}");

            query = query
                .filter(activities::created_at.gt(date))
                .order(activities::created_at.asc());
        } else if let Some(max) = max {
            let date: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_micros(max).unwrap(),
                Utc,
            );

            log::debug!("MAXIMUM {date:#?}");

            query = query
                .filter(activities::created_at.lt(date))
                .order(activities::created_at.desc());
        } else {
            query = query.order(activities::created_at.desc());
        }

        let records = query
            .get_results::<ExtendedActivityRecord>(c)
            .unwrap_or(vec![]);

        transform_records_to_extended_activities(records)
    })
    .await
}

pub async fn revoke_activity_by_uuid(conn: Option<&Db>, uuid: String) -> Result<Activity> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::update(activities::table.filter(activities::uuid.eq(uuid)))
                    .set(activities::revoked.eq(true))
                    .get_result::<Activity>(c)
                    .map_err(anyhow::Error::msg)
            })
            .await
        }
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            diesel::update(activities::table.filter(activities::uuid.eq(uuid)))
                .set(activities::revoked.eq(true))
                .get_result::<Activity>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}

pub async fn revoke_activity_by_apid(conn: Option<&Db>, ap_id: String) -> Result<Activity> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::update(activities::table.filter(activities::ap_id.eq(ap_id)))
                    .set(activities::revoked.eq(true))
                    .get_result::<Activity>(c)
                    .map_err(anyhow::Error::msg)
            })
            .await
        }
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            diesel::update(activities::table.filter(activities::ap_id.eq(ap_id)))
                .set(activities::revoked.eq(true))
                .get_result::<Activity>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}
