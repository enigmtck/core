use crate::activity_pub::PUBLIC_COLLECTION;
use crate::db::Db;
use crate::helper::{get_activity_ap_id_from_uuid, get_ap_id_from_username};
use crate::models::pg::parameter_generator;
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
use super::remote_actors::RemoteActor;
use crate::models::activities::{
    transform_records_to_extended_activities, ExtendedActivity, ExtendedActivityRecord,
    NewActivityCc, NewActivityTo, TimelineFilters, TimelineView,
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

pub async fn get_activities_coalesced(
    conn: &Db,
    limit: i32,
    min: Option<i64>,
    max: Option<i64>,
    profile: Option<Profile>,
    filters: Option<TimelineFilters>,
) -> Vec<CoalescedActivity> {
    use diesel::sql_types::{Array, Integer, Text, Timestamptz};

    let mut to: Vec<String> = vec![];
    let mut hashtags: Vec<String> = vec![];
    let mut date = Utc::now();
    let mut username: Option<String> = None;
    let mut conversation: Option<String> = None;

    log::debug!("IN COALESCED");
    let mut param_gen = parameter_generator();

    let mut lowercase_hashtags: Vec<String> = vec![];
    let ret = conn.run(move |c| {
        let mut query = "WITH main AS (\
                         SELECT DISTINCT ON (a.created_at) a.*,\
                         COALESCE(o.ek_uuid, NULL) AS object_uuid,\
                         COALESCE(o.as_type::text, NULL) AS object_type,\
                         COALESCE(o.as_published::text, NULL) AS object_published,\
                         COALESCE(o.as_id, NULL) AS object_id,\
                         COALESCE(trim('\"' FROM o.as_url::text), NULL) AS object_url,\
                         COALESCE(o.as_to, NULL) AS object_to,\
                         COALESCE(o.as_cc, NULL) AS object_cc,\
                         COALESCE(o.as_tag, NULL) AS object_tag,\
                         COALESCE(o.as_attributed_to, NULL) AS object_attributed_to,\
                         COALESCE(o.as_in_reply_to, NULL) AS object_in_reply_to,\
                         COALESCE(o.as_content, NULL) AS object_content,\
                         COALESCE(o.ap_conversation, NULL) AS object_conversation,\
                         COALESCE(o.as_attachment, NULL) AS object_attachment,\
                         COALESCE(o.as_summary, NULL) AS object_summary,\
                         COALESCE(o.as_end_time, NULL) AS object_end_time,\
                         COALESCE(o.as_one_of, NULL) AS object_one_of,\
                         COALESCE(o.as_any_of, NULL) AS object_any_of,\
                         COALESCE(o.ap_voters_count, NULL) AS object_voters_count,\
                         COALESCE(o.ap_sensitive, false) AS object_sensitive,\
                         COALESCE(o.ek_metadata, NULL) AS object_metadata ".to_string();

        query.push_str(
            "FROM activities a \
             LEFT JOIN objects o ON (o.id = a.target_object_id) \
             LEFT JOIN profiles p ON (a.profile_id = p.id) \
             WHERE a.kind IN ('announce','create') ");

        if let Some(filters) = filters.clone() {
            if filters.conversation.is_some() {
                conversation = filters.conversation;
                query.push_str(&format!("AND o.ap_conversation = {} ", param_gen()));
            } else if filters.username.is_none() {
                // The logic here is that if there is a username, we want replies and top posts,
                // so we don't use a condition. If there isn't, then we just want top posts
                query.push_str("AND NOT a.reply ");
            }
        }

        query.push_str(&format!(
             "AND NOT a.revoked \
              AND (a.target_object_id IS NOT NULL) \
              AND (a.ap_to ?| {} OR a.cc ?| {}) ",
            param_gen(), param_gen()
        ));

        // Add date filtering to the subquery
        if let Some(min) = min {
            if min != 0 {
                date = DateTime::from_naive_utc_and_offset(
                    NaiveDateTime::from_timestamp_micros(min).unwrap(),
                    Utc,
                );

                query.push_str(&format!("AND a.created_at > {} ", param_gen()));
            }
        } else if let Some(max) = max {
            date = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_micros(max).unwrap(),
                Utc,
            );

            query.push_str(&format!("AND a.created_at < {} ", param_gen()));
        }

        // Add filters based on the provided options
        if let Some(filters) = filters {
            if filters.username.is_some() {
                to.extend((*PUBLIC_COLLECTION).clone());
                query.push_str(&format!("AND p.username = {} ", param_gen()));
                username = filters.username;
            } else {
                match filters.view {
                    TimelineView::Global => {
                        to.extend((*PUBLIC_COLLECTION).clone());
                    }
                    TimelineView::Local => {
                        to.extend((*PUBLIC_COLLECTION).clone());
                        query.push_str("AND o.uuid IS NOT NULL ");
                    }
                    TimelineView::Home(leaders) => {
                        log::debug!("LEADERS\n{leaders:#?} ");
                        if let Some(profile) = profile.clone() {
                            let ap_id = get_ap_id_from_username(profile.username.clone());
                            to.extend(vec![ap_id]);
                            to.extend(leaders);
                        }
                    }
                }
            }

            hashtags.extend(filters.hashtags);
            if !hashtags.is_empty() {
                query.push_str(&format!("AND o.ek_hashtags ?| {} ", param_gen()));
            }
        }

        if min.is_some() && min.unwrap() == 0 {
            query.push_str(&format!(
                "ORDER BY created_at ASC LIMIT {}), ",
                param_gen()
            ));
        } else {
            query.push_str(&format!(
                "ORDER BY created_at DESC LIMIT {}), ",
                param_gen()
            ));
        }

        if profile.is_some() {
            query.push_str(&format!(
                "announced AS (\
                 SELECT m.id, MAX(a.uuid) AS object_announced \
                 FROM main m \
                 LEFT JOIN activities a ON (a.target_ap_id = m.object_id \
                 AND NOT a.revoked \
                 AND a.kind = 'announce' \
                 AND  a.profile_id = {}) \
                 GROUP BY m.id), \
                 liked AS (\
                 SELECT m.id, MAX(a.uuid) AS object_liked \
                 FROM main m \
                 LEFT JOIN activities a ON (a.target_ap_id = m.object_id \
                 AND NOT a.revoked \
                 AND a.kind = 'like' \
                 AND  a.profile_id = {}) \
                 GROUP BY m.id) ",
            param_gen(), param_gen()));
        } else {
            query.push_str("announced AS (\
                            SELECT m.id, NULL AS object_announced \
                            FROM main m), \
                            liked AS (\
                            SELECT m.id, NULL AS object_liked \
                            FROM main m) ");
        }

        query.push_str(" SELECT DISTINCT m.*, \
                        COALESCE(JSONB_AGG(a.actor) \
                        FILTER (WHERE a.actor IS NOT NULL AND a.kind = 'announce'), '[]') \
                        AS object_announcers, \
                        COALESCE(JSONB_AGG(a.actor) \
                        FILTER (WHERE a.actor IS NOT NULL AND a.kind = 'like'), '[]') \
                        AS object_likers, \
                        announced.object_announced, \
                        liked.object_liked ");

        query.push_str("FROM main m \
                        LEFT JOIN activities a ON (a.target_ap_id = m.object_id AND NOT a.revoked AND (a.kind = 'announce' OR a.kind = 'like')) \
                        LEFT JOIN announced ON m.id = announced.id \
                        LEFT JOIN liked ON m.id = liked.id \
                        GROUP BY m.id, m.created_at, m.updated_at, m.profile_id, \
                        m.kind, m.uuid, m.actor, m.ap_to, m.cc, m.target_note_id, m.target_remote_note_id, \
                        m.target_profile_id, m.target_activity_id, m.target_ap_id, m.target_remote_actor_id, \
                        m.revoked, m.ap_id, m.target_remote_question_id, m.reply, m.raw, m.target_object_id, \
                        m.object_uuid, m.object_type, \
                        m.object_published, m.object_id, m.object_url, m.object_to, m.object_cc, m.object_tag, \
                        m.object_attributed_to, m.object_in_reply_to, m.object_content, m.object_conversation, \
                        m.object_attachment, m.object_summary, m.object_end_time, m.object_one_of, \
                        m.object_any_of, m.object_voters_count, m.object_sensitive, m.raw, m.object_metadata, \
                        announced.object_announced, liked.object_liked \
                        ORDER BY m.created_at DESC");

        log::debug!("COALESCED QUERY\n{query:#?}");

        let mut query = sql_query(query).into_boxed();

        if let Some(conversation) = conversation {
            log::debug!("SETTING CONVERSATION: |{conversation}|");
            query = query.bind::<Text, _>(conversation);
        }

        log::debug!("SETTING TO: |{to:#?}|");
        query = query.bind::<Array<Text>, _>(&to);
        query = query.bind::<Array<Text>, _>(&to);

        if (min.is_some() && min.unwrap() != 0) || max.is_some() {
            log::debug!("SETTING DATE: |{date}|");
            query = query.bind::<Timestamptz, _>(&date);
        }

        if let Some(username) = username {
            log::debug!("SETTING USERNAME: |{username}|");
            query = query.bind::<Text, _>(username);
        }

        if !hashtags.is_empty() {
            log::debug!("SETTING HASHTAGS: |{hashtags:#?}|");
            lowercase_hashtags.extend(hashtags.iter().map(|hashtag| hashtag.to_lowercase()));
            query = query.bind::<Array<Text>, _>(&lowercase_hashtags);
            //query = query.bind::<Array<Text>, _>(&hashtags);
        }

        log::debug!("SETTING LIMIT: |{limit}|");
        query = query.bind::<Integer, _>(&limit);

        let id;
        if let Some(profile) = profile {
            id = profile.id;
            query = query.bind::<Integer, _>(&id);
            query = query.bind::<Integer, _>(&id);
        }

        query.load::<CoalescedActivity>(c).expect("bad sql query")
    })
        .await;

    log::debug!("ACTIVITIES VEC\n{ret:#?}");

    ret
}

pub async fn get_activities_coalesced_old(
    conn: &Db,
    limit: i32,
    min: Option<i64>,
    max: Option<i64>,
    profile: Option<Profile>,
    filters: Option<TimelineFilters>,
) -> Vec<CoalescedActivity> {
    use diesel::sql_types::{Array, Integer, Text, Timestamptz};

    let mut to: Vec<String> = vec![];
    let mut hashtags: Vec<String> = vec![];
    let mut date = Utc::now();
    let mut username: Option<String> = None;
    let mut conversation: Option<String> = None;

    log::debug!("IN COALESCED");
    let mut param_gen = parameter_generator();

    let ret = conn.run(move |c| {
        let mut query = "WITH main AS (\
                         SELECT DISTINCT ON (a.created_at) a.*,\
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
                         COALESCE(r.ap_sensitive, q.ap_sensitive) AS object_sensitive, \
                         COALESCE(r.metadata, NULL) AS object_metadata ".to_string();

        if let Some(filters) = filters.clone() {
            if !filters.hashtags.is_empty() {
                query.push_str(
                    "FROM remote_note_hashtags h \
                     INNER JOIN activities a ON (a.target_remote_note_id = h.remote_note_id) ");
            } else {
                query.push_str("FROM activities a ");
            }
        }

        query.push_str(
            "LEFT JOIN notes n ON (n.id = a.target_note_id) \
             LEFT JOIN remote_questions q ON (q.id = a.target_remote_question_id) \
             LEFT JOIN remote_notes r ON (r.id = a.target_remote_note_id) \
             LEFT JOIN profiles p ON (a.profile_id = p.id) \
             WHERE a.kind IN ('announce','create') ");

        if let Some(filters) = filters.clone() {
            if filters.conversation.is_some() {
                conversation = filters.conversation;
                query.push_str(&format!("AND r.conversation = {} ", param_gen()));
            } else if filters.username.is_none() {
                // The logic here is that if there is a username, we want replies and top posts,
                // so we don't use a condition. If there isn't, then we just want top posts
                query.push_str("AND NOT a.reply ");
            }
        }

        query.push_str(&format!(
             "AND NOT a.revoked \
              AND (a.target_note_id IS NOT NULL OR a.target_remote_note_id IS NOT NULL \
              OR a.target_remote_question_id IS NOT NULL) \
              AND (a.ap_to ?| {} OR a.cc ?| {}) ",
            param_gen(), param_gen()
        ));

        // Add date filtering to the subquery
        if let Some(min) = min {
            if min != 0 {
                date = DateTime::from_naive_utc_and_offset(
                    NaiveDateTime::from_timestamp_micros(min).unwrap(),
                    Utc,
                );

                query.push_str(&format!("AND a.created_at > {} ", param_gen()));
            }
        } else if let Some(max) = max {
            date = DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_micros(max).unwrap(),
                Utc,
            );

            query.push_str(&format!("AND a.created_at < {} ", param_gen()));
        }

        // Add filters based on the provided options
        if let Some(filters) = filters {
            if filters.username.is_some() {
                to.extend((*PUBLIC_COLLECTION).clone());
                query.push_str(&format!("AND p.username = {} ", param_gen()));
                username = filters.username;
            } else {
                match filters.view {
                    TimelineView::Global => {
                        to.extend((*PUBLIC_COLLECTION).clone());
                    }
                    TimelineView::Local => {
                        to.extend((*PUBLIC_COLLECTION).clone());
                        query.push_str("AND a.target_note_id IS NOT NULL ");
                    }
                    TimelineView::Home(leaders) => {
                        log::debug!("LEADERS\n{leaders:#?} ");
                        if let Some(profile) = profile.clone() {
                            let ap_id = get_ap_id_from_username(profile.username.clone());
                            to.extend(vec![ap_id]);
                            to.extend(leaders);
                        }
                    }
                }
            }

            hashtags.extend(filters.hashtags);
            if !hashtags.is_empty() {
                query.push_str(&format!("AND h.hashtag = ANY({}) ", param_gen()));
            }
        }

        if min.is_some() && min.unwrap() == 0 {
            query.push_str(&format!(
                "ORDER BY created_at ASC LIMIT {}), ",
                param_gen()
            ));
        } else {
            query.push_str(&format!(
                "ORDER BY created_at DESC LIMIT {}), ",
                param_gen()
            ));
        }

        if profile.is_some() {
            query.push_str(&format!(
                "announced AS (\
                 SELECT m.id, MAX(a.uuid) AS object_announced \
                 FROM main m \
                 LEFT JOIN activities a ON (a.target_ap_id = m.object_id \
                 AND NOT a.revoked \
                 AND a.kind = 'announce' \
                 AND  a.profile_id = {}) \
                 GROUP BY m.id), \
                 liked AS (\
                 SELECT m.id, MAX(a.uuid) AS object_liked \
                 FROM main m \
                 LEFT JOIN activities a ON (a.target_ap_id = m.object_id \
                 AND NOT a.revoked \
                 AND a.kind = 'like' \
                 AND  a.profile_id = {}) \
                 GROUP BY m.id) ",
            param_gen(), param_gen()));
        } else {
            query.push_str("announced AS (\
                            SELECT m.id, NULL AS object_announced \
                            FROM main m), \
                            liked AS (\
                            SELECT m.id, NULL AS object_liked \
                            FROM main m) ");
        }

        query.push_str(" SELECT DISTINCT m.*, \
                        COALESCE(JSONB_AGG(a.actor) \
                        FILTER (WHERE a.actor IS NOT NULL AND a.kind = 'announce'), '[]') \
                        AS object_announcers, \
                        COALESCE(JSONB_AGG(a.actor) \
                        FILTER (WHERE a.actor IS NOT NULL AND a.kind = 'like'), '[]') \
                        AS object_likers, \
                        announced.object_announced, \
                        liked.object_liked ");

        query.push_str("FROM main m \
                        LEFT JOIN activities a ON (a.target_ap_id = m.object_id AND NOT a.revoked AND (a.kind = 'announce' OR a.kind = 'like')) \
                        LEFT JOIN announced ON m.id = announced.id \
                        LEFT JOIN liked ON m.id = liked.id \
                        GROUP BY m.id, m.created_at, m.updated_at, m.profile_id, \
                        m.kind, m.uuid, m.actor, m.ap_to, m.cc, m.target_note_id, m.target_remote_note_id, \
                        m.target_profile_id, m.target_activity_id, m.target_ap_id, m.target_remote_actor_id, \
                        m.revoked, m.ap_id, m.target_remote_question_id, m.reply, m.raw, m.target_object_id, \
                        m.object_uuid, m.object_type, \
                        m.object_published, m.object_id, m.object_url, m.object_to, m.object_cc, m.object_tag, \
                        m.object_attributed_to, m.object_in_reply_to, m.object_content, m.object_conversation, \
                        m.object_attachment, m.object_summary, m.object_end_time, m.object_one_of, \
                        m.object_any_of, m.object_voters_count, m.object_sensitive, m.object_metadata, \
                        announced.object_announced, liked.object_liked \
                        ORDER BY m.created_at DESC");

        log::debug!("COALESCED QUERY\n{query:#?}");

        let mut query = sql_query(query).into_boxed();

        if let Some(conversation) = conversation {
            log::debug!("SETTING CONVERSATION: |{conversation}|");
            query = query.bind::<Text, _>(conversation);
        }

        log::debug!("SETTING TO: |{to:#?}|");
        query = query.bind::<Array<Text>, _>(&to);
        query = query.bind::<Array<Text>, _>(&to);

        if (min.is_some() && min.unwrap() != 0) || max.is_some() {
            log::debug!("SETTING DATE: |{date}|");
            query = query.bind::<Timestamptz, _>(&date);
        }

        if let Some(username) = username {
            log::debug!("SETTING USERNAME: |{username}|");
            query = query.bind::<Text, _>(username);
        }

        if !hashtags.is_empty() {
            log::debug!("SETTING HASHTAGS: |{hashtags:#?}|");
            query = query.bind::<Array<Text>, _>(&hashtags);
        }

        log::debug!("SETTING LIMIT: |{limit}|");
        query = query.bind::<Integer, _>(&limit);

        let id;
        if let Some(profile) = profile {
            id = profile.id;
            query = query.bind::<Integer, _>(&id);
            query = query.bind::<Integer, _>(&id);
        }

        query.load::<CoalescedActivity>(c).expect("bad sql query")
    })
        .await;

    log::debug!("ACTIVITIES VEC\n{ret:#?}");

    ret
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
