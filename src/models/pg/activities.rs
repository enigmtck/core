use crate::activity_pub::{ApActivity, PUBLIC_COLLECTION};
use crate::db::Db;
use crate::helper::{get_activity_ap_id_from_uuid, get_ap_id_from_username};
use crate::models::activities::{get_activity, ExtendedActivity};
use crate::models::pg::parameter_generator;
use crate::schema::{activities, actors, objects, olm_sessions, vault};
use crate::POOL;
use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::pg::Pg;
use diesel::query_builder::{BoxedSqlQuery, SqlQuery};
use diesel::sql_types::Nullable;
use diesel::{prelude::*, sql_query};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::actors::Actor;
use super::coalesced_activity::CoalescedActivity;
use super::objects::{Object, ObjectType};
use super::olm_sessions::OlmSession;
use crate::models::activities::{TimelineFilters, TimelineView};

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
    pub target_activity_id: Option<i32>,
    pub target_ap_id: Option<String>,
    pub revoked: bool,
    pub ap_id: Option<String>,
    pub reply: bool,
    pub raw: Option<Value>,
    pub target_object_id: Option<i32>,
    pub actor_id: Option<i32>,
    pub target_actor_id: Option<i32>,
    pub log: Option<Value>,
    pub instrument: Option<Value>,
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
            target_activity_id: None,
            target_ap_id: None,
            revoked: false,
            ap_id: Some(get_activity_ap_id_from_uuid(uuid)),
            reply: false,
            raw: None,
            target_object_id: None,
            actor_id: None,
            target_actor_id: None,
            log: Some(json!([])),
            instrument: None,
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
    pub kind: ActivityType,
    pub uuid: String,
    pub actor: String,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub target_activity_id: Option<i32>,
    pub target_ap_id: Option<String>,
    pub revoked: bool,
    pub ap_id: Option<String>,
    pub reply: bool,
    pub raw: Option<Value>,
    pub target_object_id: Option<i32>,
    pub actor_id: Option<i32>,
    pub target_actor_id: Option<i32>,
    pub log: Option<Value>,
    pub instrument: Option<Value>,
}

impl Activity {
    pub async fn extend(&self, conn: &Db) -> Option<ExtendedActivity> {
        get_activity(Some(conn), self.id).await
    }
}

#[derive(Default, Debug)]
struct QueryParams {
    min: Option<i64>,
    max: Option<i64>,
    to: Vec<String>,
    from: Vec<String>,
    hashtags: Vec<String>,
    date: DateTime<Utc>,
    username: Option<String>,
    conversation: Option<String>,
    limit: i32,
    query: Option<String>,
    activity_as_id: Option<String>,
    activity_uuid: Option<String>,
    activity_id: Option<i32>,
    direct: bool,
}

fn query_initial_block() -> String {
    "WITH main AS (\
     SELECT DISTINCT ON (a.created_at) a.*,\
     a2.created_at AS recursive_created_at,\
     a2.updated_at AS recursive_updated_at,\
     a2.kind AS recursive_kind,\
     a2.uuid AS recursive_uuid,\
     a2.actor AS recursive_actor,\
     a2.ap_to AS recursive_ap_to,\
     a2.cc AS recursive_cc,\
     a2.target_activity_id AS recursive_target_activity_id,\
     a2.target_ap_id AS recursive_target_ap_id,\
     a2.revoked AS recursive_revoked,\
     a2.ap_id AS recursive_ap_id,\
     a2.reply AS recursive_reply,\
     a2.target_object_id AS recursive_target_object_id,\
     a2.actor_id AS recursive_actor_id,\
     a2.target_actor_id AS recursive_target_actor_id,\
     a2.instrument AS recursive_instrument,\
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
     COALESCE(o.ek_profile_id, o2.ek_profile_id) AS object_profile_id,\
     COALESCE(o.ek_instrument, o2.ek_instrument) AS object_instrument,\
     COALESCE(ta.created_at, ta2.created_at) AS actor_created_at,\
     COALESCE(ta.updated_at, ta2.updated_at) AS actor_updated_at,\
     COALESCE(ta.ek_uuid, ta2.ek_uuid) AS actor_uuid,\
     COALESCE(ta.ek_username, ta2.ek_username) AS actor_username,\
     COALESCE(ta.ek_summary_markdown, ta2.ek_summary_markdown) AS actor_summary_markdown,\
     COALESCE(ta.ek_avatar_filename, ta2.ek_avatar_filename) AS actor_avatar_filename,\
     COALESCE(ta.ek_banner_filename, ta2.ek_banner_filename) AS actor_banner_filename,\
     COALESCE(ta.ek_private_key, ta2.ek_private_key) AS actor_private_key,\
     COALESCE(ta.ek_password, ta2.ek_password) AS actor_password,\
     COALESCE(ta.ek_client_public_key, ta2.ek_client_public_key) AS actor_client_public_key,\
     COALESCE(ta.ek_client_private_key, ta2.ek_client_private_key) AS actor_client_private_key,\
     COALESCE(ta.ek_salt, ta2.ek_salt) AS actor_salt,\
     COALESCE(ta.ek_olm_pickled_account, ta2.ek_olm_pickled_account) AS actor_olm_pickled_account,\
     COALESCE(ta.ek_olm_pickled_account_hash, ta2.ek_olm_pickled_account_hash) AS actor_olm_pickled_account_hash,\
     COALESCE(ta.ek_olm_identity_key, ta2.ek_olm_identity_key) AS actor_olm_identity_key,\
     COALESCE(ta.ek_webfinger, ta2.ek_webfinger) AS actor_webfinger,\
     COALESCE(ta.ek_checked_at, ta2.ek_checked_at) AS actor_checked_at,\
     COALESCE(ta.ek_hashtags, ta2.ek_hashtags) AS actor_hashtags,\
     COALESCE(ta.as_type, ta2.as_type) AS actor_type,\
     COALESCE(ta.as_context, ta2.as_context) AS actor_context,\
     COALESCE(ta.as_id, ta2.as_id) AS actor_as_id,\
     COALESCE(ta.as_name, ta2.as_name) AS actor_name,\
     COALESCE(ta.as_preferred_username, ta2.as_preferred_username) AS actor_preferred_username,\
     COALESCE(ta.as_summary, ta2.as_summary) AS actor_summary,\
     COALESCE(ta.as_inbox, ta2.as_inbox) AS actor_inbox,\
     COALESCE(ta.as_outbox, ta2.as_outbox) AS actor_outbox,\
     COALESCE(ta.as_followers, ta2.as_followers) AS actor_followers,\
     COALESCE(ta.as_following, ta2.as_following) AS actor_following,\
     COALESCE(ta.as_liked, ta2.as_liked) AS actor_liked,\
     COALESCE(ta.as_public_key, ta2.as_public_key) AS actor_public_key,\
     COALESCE(ta.as_featured, ta2.as_featured) AS actor_featured,\
     COALESCE(ta.as_featured_tags, ta2.as_featured_tags) AS actor_featured_tags,\
     COALESCE(ta.as_url, ta2.as_url) AS actor_url,\
     COALESCE(ta.as_published, ta2.as_published) AS actor_published,\
     COALESCE(ta.as_tag, ta2.as_tag) AS actor_tag,\
     COALESCE(ta.as_attachment, ta2.as_attachment) AS actor_attachment,\
     COALESCE(ta.as_endpoints, ta2.as_endpoints) AS actor_endpoints,\
     COALESCE(ta.as_icon, ta2.as_icon) AS actor_icon,\
     COALESCE(ta.as_image, ta2.as_image) AS actor_image,\
     COALESCE(ta.as_also_known_as, ta2.as_also_known_as) AS actor_also_known_as,\
     COALESCE(ta.as_discoverable, ta2.as_discoverable) AS actor_discoverable,\
     COALESCE(ta.ap_capabilities, ta2.ap_capabilities) AS actor_capabilities,\
     COALESCE(ta.ek_keys, ta2.ek_keys) AS actor_keys,\
     COALESCE(ta.ek_last_decrypted_activity, ta2.ek_last_decrypted_activity) AS actor_last_decrypted_activity,\
     COALESCE(ta.ap_manually_approves_followers, ta2.ap_manually_approves_followers) AS actor_manually_approves_followers ".to_string()
}

fn query_end_block(mut query: String) -> String {
    query.push_str(
        " SELECT DISTINCT m.*, \
         COALESCE(JSONB_AGG(jsonb_build_object('id', ac.as_id, 'name', ac.as_name, 'tag', ac.as_tag, 'url', ac.as_url,\
         'icon', ac.as_icon, 'preferredUsername', ac.as_preferred_username)) \
         FILTER (WHERE a.actor IS NOT NULL AND a.kind = 'announce'), '[]') \
         AS object_announcers, \
         COALESCE(JSONB_AGG(jsonb_build_object('id', ac.as_id, 'name', ac.as_name, 'tag', ac.as_tag, 'url', ac.as_url,\
         'icon', ac.as_icon, 'preferredUsername', ac.as_preferred_username)) \
         FILTER (WHERE a.actor IS NOT NULL AND a.kind = 'like'), '[]') \
         AS object_likers, \
         JSONB_AGG(DISTINCT jsonb_build_object('id', ac2.as_id, 'name', ac2.as_name, 'tag', ac2.as_tag, 'url', ac2.as_url,\
         'icon', ac2.as_icon, 'preferredUsername', ac2.as_preferred_username)) AS object_attributed_to_profiles, \
         announced.object_announced, \
         liked.object_liked, \
         vaulted.*, \
         olm.* \
         FROM main m \
         LEFT JOIN actors ac2 ON (m.object_attributed_to ?| ARRAY[ac2.as_id]) \
         LEFT JOIN activities a \
         ON (a.target_ap_id = m.object_as_id AND NOT a.revoked AND (a.kind = 'announce' OR a.kind = 'like')) \
         LEFT JOIN actors ac ON (ac.as_id = a.actor) \
         LEFT JOIN announced ON m.id = announced.id \
         LEFT JOIN liked ON m.id = liked.id \
         LEFT JOIN vaulted ON m.id = vaulted.vault_activity_id \
         LEFT JOIN olm ON olm.olm_conversation = m.object_conversation \
         GROUP BY m.id, m.created_at, m.updated_at, m.kind, m.uuid, m.actor, m.ap_to, m.cc,\
         m.target_activity_id, m.target_ap_id, m.revoked, m.ap_id, m.reply, m.instrument, m.recursive_created_at,\
         m.recursive_updated_at, m.recursive_kind, m.recursive_uuid, m.recursive_actor,\
         m.recursive_ap_to, m.recursive_cc, m.recursive_target_activity_id, m.recursive_target_ap_id,\
         m.recursive_revoked, m.recursive_ap_id, m.recursive_reply, m.recursive_target_object_id, m.recursive_actor_id,\
         m.recursive_target_actor_id, m.recursive_instrument, m.object_created_at,m.object_updated_at, m.object_uuid,\
         m.object_type, m.object_published, m.object_as_id, m.object_url, m.object_to, m.object_cc, m.object_tag,\
         m.object_attributed_to, m.object_in_reply_to, m.object_content, m.object_conversation, m.object_attachment,\
         m.object_summary, m.object_end_time, m.object_one_of, m.object_any_of, m.object_voters_count,\
         m.object_sensitive, m.object_metadata, m.object_profile_id, m.object_instrument, m.raw, m.actor_id,\
         m.target_actor_id, m.log,\
         m.target_object_id, m.actor_created_at, m.actor_updated_at, m.actor_uuid, m.actor_username,\
         m.actor_summary_markdown, m.actor_avatar_filename, m.actor_banner_filename, m.actor_private_key,\
         m.actor_password, m.actor_client_public_key, m.actor_client_private_key, m.actor_salt,\
         m.actor_olm_pickled_account, m.actor_olm_pickled_account_hash, m.actor_olm_identity_key, m.actor_webfinger,\
         m.actor_checked_at, m.actor_hashtags, m.actor_type, m.actor_context, m.actor_as_id, m.actor_name,\
         m.actor_preferred_username, m.actor_summary, m.actor_inbox, m.actor_outbox, m.actor_followers,\
         m.actor_following, m.actor_liked, m.actor_public_key, m.actor_featured, m.actor_featured_tags, m.actor_url,\
         m.actor_published, m.actor_tag, m.actor_attachment, m.actor_endpoints, m.actor_icon, m.actor_image,\
         m.actor_also_known_as, m.actor_discoverable, m.actor_capabilities, m.actor_keys,\
         m.actor_last_decrypted_activity, m.actor_manually_approves_followers,\
         announced.object_announced, liked.object_liked, vaulted.vault_id,\
         vaulted.vault_created_at, vaulted.vault_updated_at, vaulted.vault_uuid, vaulted.vault_owner_as_id,\
         vaulted.vault_activity_id, vaulted.vault_data, olm.olm_data, olm.olm_hash, olm.olm_conversation,\
         olm.olm_created_at, olm.olm_updated_at, olm.olm_owner, olm.olm_uuid, olm.olm_owner_id \
         ORDER BY m.created_at DESC");
    query
}

#[allow(clippy::too_many_arguments)]
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

    let mut param_gen = parameter_generator();

    let mut query = query_initial_block();

    query.push_str(
        "FROM activities a \
         LEFT JOIN objects o ON (o.id = a.target_object_id) \
         LEFT JOIN actors ta ON (ta.id = a.target_actor_id) \
         LEFT JOIN activities a2 ON (a.target_activity_id = a2.id) \
         LEFT JOIN objects o2 ON (a2.target_object_id = o2.id) \
         LEFT JOIN actors ta2 ON (ta2.id = a2.target_actor_id) \
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
            query.push_str(
                "WHERE a.kind IN ('announce','create') \
                 AND NOT o.as_type IN ('tombstone') ",
            );
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
                query.push_str(&format!("AND ac.ek_username = {} ", param_gen()));
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
                        if let Some(profile) = profile.clone() {
                            params.to.extend(vec![profile.as_id]);
                            params.to.extend(leaders);
                        }
                    }
                    TimelineView::Direct => {
                        if let Some(profile) = profile.clone() {
                            params.direct = true;
                            params.to.extend(vec![profile.as_id.clone()]);
                            params.from.extend(vec![profile.as_id]);
                            query.push_str(&format!(
                                "AND NOT (a.ap_to ?| {} OR a.cc ?| {}) ",
                                param_gen(),
                                param_gen(),
                            ));
                        }
                    }
                }
            }

            if !params.to.is_empty() && params.from.is_empty() {
                query.push_str(&format!(
                    "AND (a.ap_to ?| {} OR a.cc ?| {}) ",
                    param_gen(),
                    param_gen()
                ));
            } else if !params.to.is_empty() && !params.from.is_empty() {
                query.push_str(&format!(
                    "AND (a.ap_to ?| {} OR a.cc ?| {} OR a.actor = ANY({})) ",
                    param_gen(),
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
             SELECT m.id, a.ap_id AS object_liked \
             FROM main m \
             LEFT JOIN activities a ON (a.target_ap_id = m.object_as_id \
             AND NOT a.revoked \
             AND a.kind = 'like' \
             AND  a.actor_id = {}) \
             GROUP BY m.id, a.ap_id), \
             vaulted AS (\
             SELECT v.id AS vault_id, v.created_at AS vault_created_at, v.updated_at AS vault_updated_at, \
             v.uuid AS vault_uuid, v.owner_as_id AS vault_owner_as_id, v.activity_id AS vault_activity_id, \
             v.data AS vault_data \
             FROM main m \
             LEFT JOIN vault v ON (v.activity_id = m.id \
             AND (m.actor = {} OR m.ap_to @> {}) \
             AND v.owner_as_id = {})), \
             olm AS (\
             SELECT os.session_data AS olm_data, os.session_hash AS olm_hash, os.uuid AS olm_uuid, \
             os.ap_conversation AS olm_conversation, os.created_at AS olm_created_at, \
             os.updated_at AS olm_updated_at, os.owner_as_id AS olm_owner, os.owner_id AS olm_owner_id \
             FROM main m \
             LEFT JOIN olm_sessions os ON (os.ap_conversation = m.object_conversation AND os.owner_id = {}) \
             ) ",
            param_gen(),
            param_gen(),
            param_gen(),
            param_gen(),
            param_gen(),
            param_gen(),
        ));
    } else {
        query.push_str(
            "announced AS (\
             SELECT m.id, NULL AS object_announced \
             FROM main m), \
             liked AS (\
             SELECT m.id, NULL AS object_liked \
             FROM main m), \
             vaulted AS (\
             SELECT NULL AS vault_id, NULL AS vault_created_at, NULL AS vault_updated_at, \
             NULL AS vault_uuid, NULL AS vault_owner_as_id, NULL::INT AS vault_activity_id, \
             NULL AS vault_data \
             FROM main m), \
             olm AS (\
             SELECT NULL AS olm_data, NULL AS olm_hash, NULL AS olm_uuid, NULL AS olm_conversation, \
             NULL AS olm_created_at, NULL AS olm_updated_at, NULL AS olm_owner, NULL AS olm_owner_id \
             FROM main m) ",
        );
    }

    let query = query_end_block(query);

    params.query = Some(query);

    params
}

pub async fn add_log_by_as_id(conn: &Db, as_id: String, entry: Value) -> Result<usize> {
    use diesel::sql_types::{Jsonb, Text};

    //sql_query("UPDATE activities a SET log = jsonb_insert(a.log, '{0}', $1) WHERE ap_id = $2")
    let mut query = sql_query(
        "UPDATE activities a SET log = COALESCE(a.log, '[]'::jsonb) || $1::jsonb WHERE ap_id = $2",
    )
    .into_boxed::<Pg>();
    query = query.bind::<Jsonb, _>(entry.clone());
    query = query.bind::<Text, _>(as_id.clone());

    conn.run(move |c| query.execute(c))
        .await
        .map_err(anyhow::Error::msg)
}

#[allow(clippy::too_many_arguments)]
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
    use diesel::sql_types::{Array, Integer, Jsonb, Text, Timestamptz};
    let mut query = query;

    if let Some(activity_as_id) = params.activity_as_id.clone() {
        log::debug!("SETTING ACTIVITY AS_ID: |{activity_as_id}|");
        query = query.bind::<Text, _>(activity_as_id);
    } else if let Some(uuid) = params.activity_uuid.clone() {
        log::debug!("SETTING ACTIVITY AS_ID: |{uuid}|");
        query = query.bind::<Text, _>(uuid);
    } else if let Some(id) = params.activity_id {
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

        if params.direct {
            log::debug!("SETTING DIRECT");
            query = query.bind::<Array<Text>, _>((*PUBLIC_COLLECTION).clone());
            query = query.bind::<Array<Text>, _>((*PUBLIC_COLLECTION).clone());
        }

        if !params.to.is_empty() && params.from.is_empty() {
            log::debug!("SETTING TO: |{:#?}|", &params.to);
            query = query.bind::<Array<Text>, _>(params.to.clone());
            query = query.bind::<Array<Text>, _>(params.to.clone());
        } else if !params.to.is_empty() && !params.from.is_empty() {
            log::debug!("SETTING TO: |{:#?}|", &params.to);
            log::debug!("SETTING FROM: |{:#?}|", &params.from);
            query = query.bind::<Array<Text>, _>(params.to.clone());
            query = query.bind::<Array<Text>, _>(params.to.clone());
            query = query.bind::<Array<Text>, _>(params.from.clone());
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
    let as_id;
    if let Some(profile) = profile {
        id = profile.id;
        as_id = profile.as_id.clone();
        query = query.bind::<Integer, _>(id);
        query = query.bind::<Integer, _>(id);
        query = query.bind::<Text, _>(as_id.clone());
        query = query.bind::<Jsonb, _>(json!(as_id.clone()));
        query = query.bind::<Text, _>(as_id);
        query = query.bind::<Integer, _>(id);
    }

    query
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

    Ok(activity)
}

pub async fn get_announcers(
    conn: &Db,
    min: Option<i64>,
    max: Option<i64>,
    limit: Option<u8>,
    target_ap_id: String,
) -> Vec<Actor> {
    conn.run(move |c| {
        let mut query = actors::table
            .select(actors::all_columns)
            .left_join(activities::table.on(activities::actor.eq(actors::as_id)))
            .filter(activities::kind.eq(ActivityType::Announce))
            .filter(activities::target_ap_id.eq(target_ap_id))
            .into_boxed();

        if let Some(limit) = limit {
            query = query.limit(limit.into());
        }

        if let Some(min) = min {
            let date: DateTime<Utc> = DateTime::from_timestamp_micros(min).unwrap();

            query = query.filter(activities::created_at.gt(date));
        } else if let Some(max) = max {
            let date: DateTime<Utc> = DateTime::from_timestamp_micros(max).unwrap();

            query = query.filter(activities::created_at.lt(date));
        }

        query = query.order(activities::created_at.desc());
        query.get_results(c).unwrap_or(vec![])
    })
    .await
}

pub type EncryptedActivity = (Activity, Object, Option<OlmSession>);

pub async fn get_encrypted_activities(
    conn: &Db,
    since: DateTime<Utc>,
    limit: u8,
    as_to: Value,
) -> Result<Vec<EncryptedActivity>> {
    let as_to_str = as_to
        .clone()
        .as_str()
        .ok_or(anyhow!("Failed to convert Value to String"))?
        .to_string();

    conn.run(move |c| {
        activities::table
            .inner_join(objects::table.on(objects::id.nullable().eq(activities::target_object_id)))
            .left_join(
                vault::table.on(vault::activity_id
                    .eq(activities::id)
                    .and(vault::owner_as_id.eq(as_to_str.clone()))),
            )
            .left_join(
                olm_sessions::table.on(olm_sessions::ap_conversation
                    .nullable()
                    .eq(objects::ap_conversation)
                    .and(olm_sessions::owner_as_id.eq(as_to_str))),
            )
            .filter(
                objects::as_type
                    .eq(ObjectType::EncryptedNote)
                    .and(activities::kind.eq(ActivityType::Create))
                    .and(vault::activity_id.is_null())
                    .and(activities::created_at.gt(since))
                    .and(
                        activities::ap_to
                            .contains(as_to.clone())
                            .or(activities::cc.contains(as_to)),
                    ),
            )
            .select((
                activities::all_columns,
                objects::all_columns,
                olm_sessions::all_columns.nullable(),
            ))
            .order(activities::created_at.asc())
            .limit(limit.into())
            .get_results(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn get_likers(
    conn: &Db,
    min: Option<i64>,
    max: Option<i64>,
    limit: Option<u8>,
    target_ap_id: String,
) -> Vec<Actor> {
    conn.run(move |c| {
        let mut query = actors::table
            .select(actors::all_columns)
            .left_join(activities::table.on(activities::actor.eq(actors::as_id)))
            .filter(activities::kind.eq(ActivityType::Like))
            .filter(activities::target_ap_id.eq(target_ap_id))
            .into_boxed();

        if let Some(limit) = limit {
            query = query.limit(limit.into());
        }

        if let Some(min) = min {
            let date: DateTime<Utc> = DateTime::from_timestamp_micros(min).unwrap();

            query = query.filter(activities::created_at.gt(date));
        } else if let Some(max) = max {
            let date: DateTime<Utc> = DateTime::from_timestamp_micros(max).unwrap();

            query = query.filter(activities::created_at.lt(date));
        }

        query = query.order(activities::created_at.desc());
        query.get_results(c).unwrap_or(vec![])
    })
    .await
}

pub async fn revoke_activities_by_object_as_id(conn: &Db, as_id: String) -> Result<Activity> {
    conn.run(move |c| {
        diesel::update(
            activities::table.filter(
                activities::target_ap_id.eq(as_id).and(
                    activities::kind
                        .eq(ActivityType::Create)
                        .or(activities::kind.eq(ActivityType::Announce)),
                ),
            ),
        )
        .set(activities::revoked.eq(true))
        .get_result::<Activity>(c)
        .map_err(anyhow::Error::msg)
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

pub async fn set_activity_log_by_apid(
    conn: Option<&Db>,
    ap_id: String,
    log: Value,
) -> Result<Activity> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::update(activities::table.filter(activities::ap_id.eq(ap_id)))
                    .set(activities::log.eq(log))
                    .get_result::<Activity>(c)
                    .map_err(anyhow::Error::msg)
            })
            .await
        }
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            diesel::update(activities::table.filter(activities::ap_id.eq(ap_id)))
                .set(activities::log.eq(log))
                .get_result::<Activity>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}
