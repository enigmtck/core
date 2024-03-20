use crate::activity_pub::ApAddress;
use crate::db::Db;
use crate::helper::{
    get_activity_ap_id_from_uuid, get_ap_id_from_username, get_followers_ap_id_from_username,
    get_note_ap_id_from_uuid,
};
use crate::schema::{
    activities, activities_cc, activities_to, notes, profiles, remote_actors, remote_notes,
};
use crate::{MaybeMultiple, POOL};
use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::notes::NoteLike;
use super::profiles::Profile;
use crate::models::activities::{ExtendedActivity, NewActivityCc, NewActivityTo};

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
}

pub struct NoteActivity {
    pub note: NoteLike,
    pub profile: Profile,
    pub kind: ActivityType,
}

impl From<NoteActivity> for NewActivity {
    fn from(note_activity: NoteActivity) -> Self {
        let mut activity = NewActivity {
            kind: note_activity.kind.clone(),
            uuid: uuid::Uuid::new_v4().to_string(),
            actor: get_ap_id_from_username(note_activity.profile.username.clone()),
            ap_to: serde_json::to_value(vec![ApAddress::get_public()]).ok(),
            cc: None,
            target_note_id: None,
            target_remote_note_id: None,
            target_ap_id: None,
            revoked: false,
            ..Default::default()
        };

        match note_activity.note {
            NoteLike::Note(note) => {
                if note_activity.kind == ActivityType::Like
                    || note_activity.kind == ActivityType::Announce
                {
                    activity.cc = serde_json::to_value(vec![
                        note.attributed_to,
                        get_followers_ap_id_from_username(note_activity.profile.username),
                    ])
                    .ok();
                } else {
                    activity.cc = serde_json::to_value(vec![get_followers_ap_id_from_username(
                        note_activity.profile.username,
                    )])
                    .ok();
                }
                activity.target_note_id = Some(note.id);
                activity.target_ap_id = Some(get_note_ap_id_from_uuid(note.uuid));
            }
            NoteLike::RemoteNote(remote_note) => {
                if note_activity.kind == ActivityType::Like
                    || note_activity.kind == ActivityType::Announce
                {
                    activity.cc = serde_json::to_value(vec![
                        remote_note.attributed_to,
                        get_followers_ap_id_from_username(note_activity.profile.username),
                    ])
                    .ok();
                } else {
                    activity.cc = serde_json::to_value(vec![get_followers_ap_id_from_username(
                        note_activity.profile.username,
                    )])
                    .ok();
                }
                activity.target_remote_note_id = Some(remote_note.id);
                activity.target_ap_id = Some(remote_note.ap_id);
            }
        }

        activity
    }
}

pub type UndoActivity = (Activity, ActivityType, ApAddress);
impl From<UndoActivity> for NewActivity {
    fn from((activity, kind, actor): UndoActivity) -> Self {
        NewActivity {
            kind,
            uuid: uuid::Uuid::new_v4().to_string(),
            actor: actor.to_string(),
            target_activity_id: Some(activity.id),
            target_ap_id: Some(get_activity_ap_id_from_uuid(activity.uuid)),
            revoked: false,
            ..Default::default()
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
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
}

#[derive(Identifiable, Queryable, AsChangeset, Associations, Serialize, Clone, Default, Debug)]
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

#[derive(Identifiable, Queryable, AsChangeset, Associations, Serialize, Clone, Default, Debug)]
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

pub async fn get_activity_by_kind_profile_id_and_target_ap_id(
    conn: &Db,
    kind: ActivityType,
    profile_id: i32,
    target_ap_id: String,
) -> Option<ExtendedActivity> {
    conn.run(move |c| {
        activities::table
            .filter(activities::revoked.eq(false))
            .filter(activities::kind.eq(kind))
            .filter(activities::profile_id.eq(profile_id))
            .filter(activities::target_ap_id.eq(target_ap_id))
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
            .first::<ExtendedActivity>(c)
    })
    .await
    .ok()
}

pub async fn get_outbox_count_by_profile_id(conn: &Db, profile_id: i32) -> Option<i64> {
    conn.run(move |c| {
        activities::table
            .filter(activities::revoked.eq(false))
            .filter(activities::profile_id.eq(profile_id))
            .filter(
                activities::kind
                    .eq(ActivityType::Create)
                    .or(activities::kind.eq(ActivityType::Announce)),
            )
            .count()
            .get_result::<i64>(c)
    })
    .await
    .ok()
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

        query.get_results::<ExtendedActivity>(c)
    })
    .await
    .unwrap_or(vec![])
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
