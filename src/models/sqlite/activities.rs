use crate::db::Db;
use crate::schema::{
    activities, activities_cc, activities_to, notes, profiles, remote_actors, remote_notes,
    remote_questions,
};
use crate::{MaybeMultiple, POOL};
use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::models::activities::{ExtendedActivity, NewActivityCc, NewActivityTo};

#[derive(Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq)]
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
    pub kind: String,
    pub uuid: String,
    pub actor: String,
    pub ap_to: Option<String>,
    pub cc: Option<String>,
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
}

#[derive(
    Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug, Hash, PartialEq, Eq,
)]
#[diesel(table_name = activities)]
pub struct Activity {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub profile_id: Option<i32>,
    pub kind: String,
    pub uuid: String,
    pub actor: String,
    pub ap_to: Option<String>,
    pub cc: Option<String>,
    pub target_note_id: Option<i32>,
    pub target_remote_note_id: Option<i32>,
    pub target_profile_id: Option<i32>,
    pub target_activity_id: Option<i32>,
    pub target_ap_id: Option<String>,
    pub target_remote_actor_id: Option<i32>,
    pub revoked: bool,
    pub ap_id: Option<String>,
    pub target_remote_question_id: Option<i32>,
}

#[derive(Identifiable, Queryable, AsChangeset, Associations, Serialize, Clone, Default, Debug)]
#[diesel(belongs_to(Activity, foreign_key = activity_id))]
#[diesel(table_name = activities_cc)]
pub struct ActivityCc {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub activity_id: i32,
    pub ap_id: String,
}

pub async fn create_activity_cc(conn: Option<&Db>, activity_cc: NewActivityCc) -> bool {
    log::debug!("INSERTING ACTIVITY_CC: {activity_cc:#?}");

    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(activities_cc::table)
                    .values(&activity_cc)
                    .execute(c)
                    .is_ok()
            })
            .await
        }
        None => POOL.get().map_or(false, |mut pool| {
            diesel::insert_into(activities_cc::table)
                .values(&activity_cc)
                .execute(&mut pool)
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub activity_id: i32,
    pub ap_id: String,
}

pub async fn create_activity_to(conn: Option<&Db>, activity_to: NewActivityTo) -> bool {
    log::debug!("INSERTING ACTIVITY_TO: {activity_to:#?}");

    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(activities_to::table)
                    .values(&activity_to)
                    .execute(c)
                    .is_ok()
            })
            .await
        }
        None => POOL.get().map_or(false, |mut pool| {
            diesel::insert_into(activities_to::table)
                .values(&activity_to)
                .execute(&mut pool)
                .is_ok()
        }),
    }
}

pub async fn create_activity(conn: Option<&Db>, activity: NewActivity) -> Result<Activity> {
    let activity = match conn {
        Some(conn) => {
            conn.run(move |c| {
                let _ = diesel::insert_into(activities::table)
                    .values(&activity)
                    .execute(c);

                activities::table
                    .order(activities::id.desc())
                    .first::<Activity>(c)
            })
            .await?
        }
        None => {
            let mut pool = POOL.get()?;
            let _ = diesel::insert_into(activities::table)
                .values(&activity)
                .execute(&mut pool);

            activities::table
                .order(activities::id.desc())
                .first::<Activity>(&mut pool)?
        }
    };

    if let Some(ap_to) = activity.clone().ap_to {
        let to: MaybeMultiple<String> = serde_json::from_str(&ap_to).map_err(|e| anyhow!(e))?;

        for to in to.multiple() {
            let _ = create_activity_to(conn, (activity.clone(), to).into()).await;
        }
    }

    if let Some(cc) = activity.clone().cc {
        let cc: MaybeMultiple<String> = serde_json::from_str(&cc).map_err(|e| anyhow!(e))?;

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
                    .eq("create".to_string())
                    .or(activities::kind.eq("announce".to_string())),
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
            .into_boxed();

        if let Some(limit) = limit {
            query = query.limit(limit.into());
        }

        if let Some(min) = min {
            let date = NaiveDateTime::from_timestamp_micros(min).unwrap();

            log::debug!("MINIMUM {date:#?}");

            query = query
                .filter(activities::created_at.gt(date))
                .order(activities::created_at.asc());
        } else if let Some(max) = max {
            let date = NaiveDateTime::from_timestamp_micros(max).unwrap();

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
        Some(conn) => conn
            .run(move |c| {
                diesel::update(activities::table.filter(activities::uuid.eq(uuid.clone())))
                    .set(activities::revoked.eq(true))
                    .execute(c)?;

                activities::table
                    .filter(activities::uuid.eq(uuid))
                    .first::<Activity>(c)
            })
            .await
            .map_err(anyhow::Error::msg),
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            diesel::update(activities::table.filter(activities::uuid.eq(uuid.clone())))
                .set(activities::revoked.eq(true))
                .execute(&mut pool)
                .map_err(anyhow::Error::msg)?;

            activities::table
                .filter(activities::uuid.eq(uuid))
                .first::<Activity>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}

pub async fn revoke_activity_by_apid(conn: Option<&Db>, ap_id: String) -> Result<Activity> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::update(activities::table.filter(activities::ap_id.eq(ap_id.clone())))
                    .set(activities::revoked.eq(true))
                    .execute(c)?;

                activities::table
                    .filter(activities::ap_id.eq(ap_id))
                    .first::<Activity>(c)
            })
            .await
            .map_err(anyhow::Error::msg),
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            diesel::update(activities::table.filter(activities::ap_id.eq(ap_id.clone())))
                .set(activities::revoked.eq(true))
                .execute(&mut pool)
                .map_err(anyhow::Error::msg)?;

            activities::table
                .filter(activities::ap_id.eq(ap_id))
                .first::<Activity>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}
