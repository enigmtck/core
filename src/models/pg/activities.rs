use crate::activity_pub::PUBLIC_COLLECTION;
use crate::db::Db;
use crate::helper::{get_activity_ap_id_from_uuid, get_ap_id_from_username};
use crate::models::pg::parameter_generator;
use crate::schema::{activities, activities_cc, activities_to, remote_actors};
use crate::{MaybeMultiple, POOL};
use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::query_builder::{BoxedSqlQuery, SqlQuery};
use diesel::sql_types::Nullable;
use diesel::{prelude::*, sql_query};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::actors::Actor;
use super::coalesced_activity::CoalescedActivity;
use super::remote_actors::RemoteActor;
use crate::models::activities::{NewActivityCc, NewActivityTo, TimelineFilters, TimelineView};

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

impl ActivityType {
    pub fn is_create(&self) -> bool {
        self == &ActivityType::Create
    }

    pub fn is_delete(&self) -> bool {
        self == &ActivityType::Delete
    }

    pub fn is_update(&self) -> bool {
        self == &ActivityType::Update
    }

    pub fn is_announce(&self) -> bool {
        self == &ActivityType::Announce
    }

    pub fn is_like(&self) -> bool {
        self == &ActivityType::Like
    }

    pub fn is_undo(&self) -> bool {
        self == &ActivityType::Undo
    }

    pub fn is_follow(&self) -> bool {
        self == &ActivityType::Follow
    }

    pub fn is_accept(&self) -> bool {
        self == &ActivityType::Accept
    }
}

#[derive(Serialize, Deserialize, Insertable, Debug, Clone, AsChangeset)]
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
    pub raw: Option<Value>,
    pub target_object_id: Option<i32>,
    pub actor_id: Option<i32>,
    pub target_actor_id: Option<i32>,
}

impl Default for NewActivity {
    fn default() -> Self {
        let uuid = uuid::Uuid::new_v4().to_string();

        NewActivity {
            kind: ActivityType::default(),
            uuid: uuid.clone(),
            actor: String::new(),
            ap_to: None,
            cc: None,
            profile_id: None,
            target_note_id: None,
            target_remote_note_id: None,
            target_profile_id: None,
            target_activity_id: None,
            target_ap_id: None,
            target_remote_actor_id: None,
            revoked: false,
            ap_id: Some(get_activity_ap_id_from_uuid(uuid)),
            target_remote_question_id: None,
            reply: false,
            raw: None,
            target_object_id: None,
            actor_id: None,
            target_actor_id: None,
        }
    }
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
    pub raw: Option<Value>,
    pub target_object_id: Option<i32>,
    pub actor_id: Option<i32>,
    pub target_actor_id: Option<i32>,
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

#[derive(Default, Debug)]
struct QueryParams {
    min: Option<i64>,
    max: Option<i64>,
    to: Vec<String>,
    hashtags: Vec<String>,
    date: DateTime<Utc>,
    username: Option<String>,
    conversation: Option<String>,
    limit: i32,
    query: Option<String>,
    activity_as_id: Option<String>,
    activity_uuid: Option<String>,
    activity_id: Option<i32>,
}

fn query_initial_block() -> String {
    "WITH main AS (\
     SELECT DISTINCT ON (a.created_at) a.*,\
     a2.created_at AS recursive_created_at,\
     a2.updated_at AS recursive_updated_at,\
     a2.profile_id AS recursive_profile_id,\
     a2.kind AS recursive_kind,\
     a2.uuid AS recursive_uuid,\
     a2.actor AS recursive_actor,\
     a2.ap_to AS recursive_ap_to,\
     a2.cc AS recursive_cc,\
     a2.target_note_id AS recursive_target_note_id,\
     a2.target_remote_note_id AS recursive_target_remote_note_id,\
     a2.target_profile_id AS recursive_target_profile_id,\
     a2.target_activity_id AS recursive_target_activity_id,\
     a2.target_ap_id AS recursive_target_ap_id,\
     a2.target_remote_actor_id AS recursive_target_remote_actor_id,\
     a2.revoked AS recursive_revoked,\
     a2.ap_id AS recursive_ap_id,\
     a2.target_remote_question_id AS recursive_target_remote_question_id,\
     a2.reply AS recursive_reply,\
     a2.target_object_id AS recursive_target_object_id,\
     a2.actor_id AS recursive_actor_id,\
     a2.target_actor_id AS recursive_target_actor_id,\
     COALESCE(o.created_at, o2.created_at) AS object_created_at,\
     COALESCE(o.updated_at, o2.updated_at) AS object_updated_at,\
     COALESCE(o.ek_uuid, o2.ek_uuid) AS object_uuid,\
     COALESCE(o.as_type, o2.as_type) AS object_type,\
     COALESCE(o.as_published, o2.as_published) AS object_published,\
     COALESCE(o.as_id, o2.as_id) AS object_as_id,\
     COALESCE(o.as_url, o2.as_url) AS object_url,\
     COALESCE(o.as_to, o2.as_to) AS object_to,\
     COALESCE(o.as_cc, o2.as_cc) AS object_cc,\
     COALESCE(o.as_tag, o2.as_tag) AS object_tag,\
     COALESCE(o.as_attributed_to, o2.as_attributed_to) AS object_attributed_to,\
     COALESCE(o.as_in_reply_to, o2.as_in_reply_to) AS object_in_reply_to,\
     COALESCE(o.as_content, o2.as_content) AS object_content,\
     COALESCE(o.ap_conversation, o2.ap_conversation) AS object_conversation,\
     COALESCE(o.as_attachment, o2.as_attachment) AS object_attachment,\
     COALESCE(o.as_summary, o2.as_summary) AS object_summary,\
     COALESCE(o.as_end_time, o2.as_end_time) AS object_end_time,\
     COALESCE(o.as_one_of, o2.as_one_of) AS object_one_of,\
     COALESCE(o.as_any_of, o2.as_any_of) AS object_any_of,\
     COALESCE(o.ap_voters_count, o2.ap_voters_count) AS object_voters_count,\
     COALESCE(o.ap_sensitive, o2.ap_sensitive) AS object_sensitive,\
     COALESCE(o.ek_metadata, o2.ek_metadata) AS object_metadata,\
     COALESCE(o.ek_profile_id, o2.ek_profile_id) AS object_profile_id "
        .to_string()
}

fn query_end_block(mut query: String) -> String {
    query.push_str(
        " SELECT DISTINCT m.*, \
         COALESCE(JSONB_AGG(a.actor) \
         FILTER (WHERE a.actor IS NOT NULL AND a.kind = 'announce'), '[]') \
         AS object_announcers, \
         COALESCE(JSONB_AGG(a.actor) \
         FILTER (WHERE a.actor IS NOT NULL AND a.kind = 'like'), '[]') \
         AS object_likers, \
         announced.object_announced, \
         liked.object_liked \
         FROM main m \
         LEFT JOIN activities a \
         ON (a.target_ap_id = m.object_as_id AND NOT a.revoked AND (a.kind = 'announce' OR a.kind = 'like')) \
         LEFT JOIN announced ON m.id = announced.id \
         LEFT JOIN liked ON m.id = liked.id \
         GROUP BY m.id, m.created_at, m.updated_at, m.profile_id, m.kind, m.uuid, m.actor, m.ap_to, m.cc,\
         m.target_note_id, m.target_remote_note_id, m.target_profile_id, m.target_activity_id, m.target_ap_id,\
         m.target_remote_actor_id, m.revoked, m.ap_id, m.target_remote_question_id, m.reply, m.recursive_created_at,\
         m.recursive_updated_at, m.recursive_profile_id, m.recursive_kind, m.recursive_uuid, m.recursive_actor,\
         m.recursive_ap_to, m.recursive_cc, m.recursive_target_note_id, m.recursive_target_remote_note_id,\
         m.recursive_target_profile_id, m.recursive_target_activity_id, m.recursive_target_ap_id,\
         m.recursive_target_remote_actor_id, m.recursive_revoked, m.recursive_ap_id,\
         m.recursive_target_remote_question_id, m.recursive_reply, m.recursive_target_object_id, m.recursive_actor_id,\
         m.recursive_target_actor_id, m.object_created_at,m.object_updated_at, m.object_uuid,\
         m.object_type, m.object_published, m.object_as_id, m.object_url, m.object_to, m.object_cc, m.object_tag,\
         m.object_attributed_to, m.object_in_reply_to, m.object_content, m.object_conversation, m.object_attachment,\
         m.object_summary, m.object_end_time, m.object_one_of, m.object_any_of, m.object_voters_count,\
         m.object_sensitive, m.object_metadata, m.object_profile_id, m.raw, m.target_object_id,\
         announced.object_announced, liked.object_liked 
         ORDER BY m.created_at DESC");
    query
}

fn build_main_query(
    filters: &Option<TimelineFilters>,
    limit: i32,
    min: Option<i64>,
    max: Option<i64>,
    profile: &Option<Actor>,
    activity_as_id: Option<String>,
    activity_uuid: Option<String>,
    activity_id: Option<i32>,
) -> QueryParams {
    let mut params = QueryParams {
        limit,
        activity_as_id,
        activity_uuid,
        activity_id,
        min,
        max,
        ..Default::default()
    };

    log::debug!("IN COALESCED");
    let mut param_gen = parameter_generator();

    let mut query = query_initial_block();

    query.push_str(
        "FROM activities a \
         LEFT JOIN objects o ON (o.id = a.target_object_id) \
         LEFT JOIN activities a2 ON (a.target_activity_id = a2.id) \
         LEFT JOIN objects o2 ON (a2.target_object_id = o2.id) \
         LEFT JOIN actors ac ON (a.actor_id = ac.id) ",
    );

    if params.activity_as_id.is_some() {
        query.push_str(&format!("WHERE a.ap_id = {}), ", param_gen()));
    } else if params.activity_uuid.is_some() {
        query.push_str(&format!("WHERE a.uuid = {}), ", param_gen()));
    } else if params.activity_id.is_some() {
        query.push_str(&format!("WHERE a.id = {}), ", param_gen()));
    } else {
        if filters.clone().and_then(|x| x.view).is_some() {
            query.push_str("WHERE a.kind IN ('announce','create') ");
        } else {
            query.push_str(
                "WHERE a.kind IN ('announce','create','undo','like','follow','accept','delete') ",
            );
        }

        if let Some(filters) = filters.clone() {
            if filters.conversation.is_some() {
                params.conversation = filters.conversation;
                query.push_str(&format!("AND o.ap_conversation = {} ", param_gen()));
            } else if filters.username.is_none() {
                // The logic here is that if there is a username, we want replies and top posts,
                // so we don't use a condition. If there isn't, then we just want top posts
                query.push_str("AND NOT a.reply ");
            }
        }

        query.push_str("AND NOT a.revoked AND (a.target_object_id IS NOT NULL OR a.target_activity_id IS NOT NULL) ");

        // Add date filtering to the subquery
        if let Some(min) = min {
            if min != 0 {
                params.date = DateTime::from_naive_utc_and_offset(
                    NaiveDateTime::from_timestamp_micros(min).unwrap(),
                    Utc,
                );

                query.push_str(&format!("AND a.created_at > {} ", param_gen()));
            }
        } else if let Some(max) = max {
            params.date = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_micros(max).unwrap(),
                Utc,
            );

            query.push_str(&format!("AND a.created_at < {} ", param_gen()));
        }

        // Add filters based on the provided options
        if let Some(filters) = filters {
            if filters.username.is_some() {
                params.to.extend((*PUBLIC_COLLECTION).clone());
                query.push_str(&format!("AND p.username = {} ", param_gen()));
                params.username = filters.username.clone();
            } else if let Some(view) = filters.view.clone() {
                match view {
                    TimelineView::Global => {
                        params.to.extend((*PUBLIC_COLLECTION).clone());
                    }
                    TimelineView::Local => {
                        params.to.extend((*PUBLIC_COLLECTION).clone());
                        query.push_str("AND o.ek_uuid IS NOT NULL ");
                    }
                    TimelineView::Home(leaders) => {
                        log::debug!("LEADERS\n{leaders:#?} ");
                        if let Some(profile) = profile.clone() {
                            if let Some(username) = profile.ek_username.clone() {
                                let ap_id = get_ap_id_from_username(username);
                                params.to.extend(vec![ap_id]);
                                params.to.extend(leaders);
                            }
                        }
                    }
                }
            }

            if !params.to.is_empty() {
                query.push_str(&format!(
                    "AND (a.ap_to ?| {} OR a.cc ?| {}) ",
                    param_gen(),
                    param_gen()
                ));
            }

            params.hashtags.extend(filters.hashtags.clone());
            if !params.hashtags.is_empty() {
                query.push_str(&format!("AND o.ek_hashtags ?| {} ", param_gen()));
            }
        }

        if min.is_some() && min.unwrap() == 0 {
            query.push_str(&format!("ORDER BY created_at ASC LIMIT {}), ", param_gen()));
        } else {
            query.push_str(&format!(
                "ORDER BY created_at DESC LIMIT {}), ",
                param_gen()
            ));
        }
    };

    if profile.is_some() {
        query.push_str(&format!(
            "announced AS (\
             SELECT m.id, MAX(a.uuid) AS object_announced \
             FROM main m \
             LEFT JOIN activities a ON (a.target_ap_id = m.object_as_id \
             AND NOT a.revoked \
             AND a.kind = 'announce' \
             AND  a.actor_id = {}) \
             GROUP BY m.id), \
             liked AS (\
             SELECT m.id, MAX(a.uuid) AS object_liked \
             FROM main m \
             LEFT JOIN activities a ON (a.target_ap_id = m.object_as_id \
             AND NOT a.revoked \
             AND a.kind = 'like' \
             AND  a.actor_id = {}) \
             GROUP BY m.id) ",
            param_gen(),
            param_gen()
        ));
    } else {
        query.push_str(
            "announced AS (\
             SELECT m.id, NULL AS object_announced \
             FROM main m), \
             liked AS (\
             SELECT m.id, NULL AS object_liked \
             FROM main m) ",
        );
    }

    let query = query_end_block(query);

    params.query = Some(query);

    params
}

pub async fn get_activities_coalesced(
    conn: &Db,
    limit: i32,
    min: Option<i64>,
    max: Option<i64>,
    profile: Option<Actor>,
    filters: Option<TimelineFilters>,
    as_id: Option<String>,
    uuid: Option<String>,
    id: Option<i32>,
) -> Vec<CoalescedActivity> {
    let params = build_main_query(&filters, limit, min, max, &profile, as_id, uuid, id);

    log::debug!("QUERY\n{params:#?}");

    let query = sql_query(params.query.clone().unwrap()).into_boxed::<diesel::pg::Pg>();
    let query = bind_params(query, params, &profile);

    conn.run(move |c| query.load::<CoalescedActivity>(c).expect("bad sql query"))
        .await
}

fn bind_params<'a>(
    query: BoxedSqlQuery<'a, diesel::pg::Pg, SqlQuery>,
    params: QueryParams,
    profile: &Option<Actor>,
) -> BoxedSqlQuery<'a, diesel::pg::Pg, SqlQuery> {
    use diesel::sql_types::{Array, Integer, Text, Timestamptz};
    let mut query = query;

    if let Some(activity_as_id) = params.activity_as_id.clone() {
        log::debug!("SETTING ACTIVITY AS_ID: |{activity_as_id}|");
        query = query.bind::<Text, _>(activity_as_id);
    } else if let Some(uuid) = params.activity_uuid.clone() {
        log::debug!("SETTING ACTIVITY AS_ID: |{uuid}|");
        query = query.bind::<Text, _>(uuid);
    } else if let Some(id) = params.activity_id.clone() {
        log::debug!("SETTING ACTIVITY AS_ID: |{id}|");
        query = query.bind::<Integer, _>(id);
    } else {
        if let Some(conversation) = params.conversation.clone() {
            log::debug!("SETTING CONVERSATION: |{conversation}|");
            query = query.bind::<Text, _>(conversation);
        }

        if (params.min.is_some() && params.min.unwrap() != 0) || params.max.is_some() {
            log::debug!("SETTING DATE: |{}|", &params.date);
            query = query.bind::<Timestamptz, _>(params.date);
        }

        if let Some(username) = params.username.clone() {
            log::debug!("SETTING USERNAME: |{username}|");
            query = query.bind::<Text, _>(username);
        }

        if !params.to.is_empty() {
            log::debug!("SETTING TO: |{:#?}|", &params.to);
            query = query.bind::<Array<Text>, _>(params.to.clone());
            query = query.bind::<Array<Text>, _>(params.to.clone());
        }

        let mut lowercase_hashtags: Vec<String> = vec![];
        if !params.hashtags.is_empty() {
            log::debug!("SETTING HASHTAGS: |{:#?}|", &params.hashtags);
            lowercase_hashtags.extend(params.hashtags.iter().map(|hashtag| hashtag.to_lowercase()));
            query = query.bind::<Array<Text>, _>(lowercase_hashtags);
        }

        log::debug!("SETTING LIMIT: |{}|", &params.limit);
        query = query.bind::<Integer, _>(params.limit);
    }

    let id;
    if let Some(profile) = profile {
        id = profile.id;
        query = query.bind::<Integer, _>(id);
        query = query.bind::<Integer, _>(id);
    }

    query
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
                    .on_conflict(activities::ap_id)
                    .do_update()
                    .set(&activity)
                    .get_result::<Activity>(c)
            })
            .await?
        }
        None => {
            let mut pool = POOL.get()?;
            diesel::insert_into(activities::table)
                .values(&activity)
                .on_conflict(activities::ap_id)
                .do_update()
                .set(&activity)
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

pub async fn get_announcers(
    conn: &Db,
    min: Option<i64>,
    max: Option<i64>,
    limit: Option<u8>,
    target_ap_id: String,
) -> Vec<RemoteActor> {
    conn.run(move |c| {
        let mut query = remote_actors::table
            .select(remote_actors::all_columns)
            .left_join(activities::table.on(activities::actor.eq(remote_actors::ap_id)))
            .filter(activities::kind.eq(ActivityType::Announce))
            .filter(activities::target_ap_id.eq(target_ap_id))
            .into_boxed();

        if let Some(limit) = limit {
            query = query.limit(limit.into());
        }

        if let Some(min) = min {
            let date: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_micros(min).unwrap(),
                Utc,
            );

            query = query.filter(activities::created_at.gt(date));
        } else if let Some(max) = max {
            let date: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_micros(max).unwrap(),
                Utc,
            );

            query = query.filter(activities::created_at.lt(date));
        }

        query = query.order(activities::created_at.desc());
        query.get_results(c).unwrap_or(vec![])
    })
    .await
}

pub async fn get_likers(
    conn: &Db,
    min: Option<i64>,
    max: Option<i64>,
    limit: Option<u8>,
    target_ap_id: String,
) -> Vec<RemoteActor> {
    conn.run(move |c| {
        let mut query = remote_actors::table
            .select(remote_actors::all_columns)
            .left_join(activities::table.on(activities::actor.eq(remote_actors::ap_id)))
            .filter(activities::kind.eq(ActivityType::Like))
            .filter(activities::target_ap_id.eq(target_ap_id))
            .into_boxed();

        if let Some(limit) = limit {
            query = query.limit(limit.into());
        }

        if let Some(min) = min {
            let date: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_micros(min).unwrap(),
                Utc,
            );

            query = query.filter(activities::created_at.gt(date));
        } else if let Some(max) = max {
            let date: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_micros(max).unwrap(),
                Utc,
            );

            query = query.filter(activities::created_at.lt(date));
        }

        query = query.order(activities::created_at.desc());
        query.get_results(c).unwrap_or(vec![])
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
