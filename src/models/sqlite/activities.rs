use crate::activity_pub::{ApActivity, ApAddress};
use crate::db::Db;
use crate::helper::{
    get_activity_ap_id_from_uuid, get_ap_id_from_username, get_followers_ap_id_from_username,
    get_note_ap_id_from_uuid,
};
use crate::schema::{
    activities, activities_cc, activities_to, notes, profiles, remote_actors, remote_notes,
};
use crate::{MaybeMultiple, MaybeReference, POOL};
use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use super::notes::NoteLike;
use super::profiles::Profile;
use super::remote_actors::RemoteActor;
use crate::models::activities::{ActivityTarget, ExtendedActivity, NewActivityCc, NewActivityTo};

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
}

pub type ApActivityTarget = (ApActivity, Option<ActivityTarget>);

impl TryFrom<ApActivityTarget> for NewActivity {
    type Error = &'static str;

    // eventually I may be able to move decomposition logic here (e.g., create target_remote_note, etc.)
    // that will require the ability to `await` on database calls
    //
    // https://blog.rust-lang.org/inside-rust/2023/05/03/stabilizing-async-fn-in-trait.html
    fn try_from((activity, target): ApActivityTarget) -> Result<Self, Self::Error> {
        let uuid = uuid::Uuid::new_v4().to_string();

        match activity {
            ApActivity::Create(create) => Ok(NewActivity {
                kind: "create".to_string(),
                uuid: uuid.clone(),
                actor: create.actor.to_string(),
                ap_to: serde_json::to_string(&create.to).ok(),
                cc: serde_json::to_string(&create.cc).ok(),
                revoked: false,
                ap_id: create
                    .id
                    .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                ..Default::default()
            }
            .link_target(target)
            .clone()),
            ApActivity::Announce(announce) => Ok(NewActivity {
                kind: "announce".to_string(),
                uuid: uuid.clone(),
                actor: announce.actor.to_string(),
                ap_to: serde_json::to_string(&announce.to).ok(),
                cc: serde_json::to_string(&announce.cc).ok(),
                target_ap_id: announce.object.reference(),
                revoked: false,
                ap_id: announce
                    .id
                    .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                ..Default::default()
            }
            .link_target(target)
            .clone()),
            ApActivity::Follow(follow) => Ok(NewActivity {
                kind: "follow".to_string(),
                uuid: uuid.clone(),
                actor: follow.actor.to_string(),
                target_ap_id: follow.object.reference(),
                target_remote_actor_id: None,
                revoked: false,
                ap_id: follow
                    .id
                    .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                ..Default::default()
            }
            .link_target(target)
            .clone()),
            ApActivity::Accept(accept) => {
                if let MaybeReference::Actual(ApActivity::Follow(follow)) = accept.object {
                    Ok(NewActivity {
                        kind: "accept".to_string(),
                        uuid: uuid.clone(),
                        actor: accept.actor.to_string(),
                        target_ap_id: follow.id,
                        revoked: false,
                        ap_id: accept
                            .id
                            .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                        ..Default::default()
                    }
                    .link_target(target)
                    .clone())
                } else {
                    Err("ACCEPT OBJECT NOT AN ACTUAL")
                }
            }
            ApActivity::Undo(undo) => match undo.object {
                MaybeReference::Actual(ApActivity::Follow(follow)) => Ok(NewActivity {
                    kind: "undo".to_string(),
                    uuid: uuid.clone(),
                    actor: undo.actor.to_string(),
                    target_ap_id: follow.id,
                    revoked: false,
                    ap_id: undo
                        .id
                        .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                    ..Default::default()
                }
                .link_target(target)
                .clone()),
                MaybeReference::Actual(ApActivity::Like(like)) => Ok(NewActivity {
                    kind: "undo".to_string(),
                    uuid: uuid.clone(),
                    actor: undo.actor.to_string(),
                    target_ap_id: like.id,
                    revoked: false,
                    ap_id: undo
                        .id
                        .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                    ..Default::default()
                }
                .link_target(target)
                .clone()),
                MaybeReference::Actual(ApActivity::Announce(announce)) => Ok(NewActivity {
                    kind: "undo".to_string(),
                    uuid: uuid.clone(),
                    actor: undo.actor.to_string(),
                    target_ap_id: announce.id,
                    revoked: false,
                    ap_id: undo
                        .id
                        .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                    ..Default::default()
                }
                .link_target(target)
                .clone()),
                _ => Err("UNDO OBJECT NOT IMPLEMENTED"),
            },
            ApActivity::Like(like) => Ok(NewActivity {
                kind: "like".to_string(),
                uuid: uuid.clone(),
                actor: like.actor.to_string(),
                target_ap_id: like.object.reference(),
                revoked: false,
                ap_id: like
                    .id
                    .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                ap_to: serde_json::to_string(&like.to).ok(),
                ..Default::default()
            }
            .link_target(target)
            .clone()),

            _ => Err("UNIMPLEMENTED ACTIVITY TYPE"),
        }
    }
}

pub type ActorActivity = (
    Option<Profile>,
    Option<RemoteActor>,
    ActivityType,
    ApAddress,
);

impl From<ActorActivity> for NewActivity {
    fn from((profile, remote_actor, kind, actor): ActorActivity) -> Self {
        let (ap_to, target_ap_id) = {
            if let Some(profile) = profile.clone() {
                (
                    Some(
                        serde_json::to_string(&vec![get_ap_id_from_username(
                            profile.username.clone(),
                        )])
                        .unwrap(),
                    ),
                    Some(get_ap_id_from_username(profile.username)),
                )
            } else if let Some(remote_actor) = remote_actor.clone() {
                (
                    Some(serde_json::to_string(&vec![remote_actor.ap_id.clone()]).unwrap()),
                    Some(remote_actor.ap_id),
                )
            } else {
                (None, None)
            }
        };

        let uuid = uuid::Uuid::new_v4().to_string();

        NewActivity {
            kind: kind.to_string().to_lowercase(),
            uuid: uuid.clone(),
            actor: actor.to_string(),
            ap_to,
            target_profile_id: profile.map(|x| x.id),
            target_ap_id,
            target_remote_actor_id: remote_actor.map(|x| x.id),
            revoked: false,
            ap_id: Some(get_activity_ap_id_from_uuid(uuid)),
            ..Default::default()
        }
    }
}

pub struct NoteActivity {
    pub note: NoteLike,
    pub profile: Profile,
    pub kind: ActivityType,
}

impl From<NoteActivity> for NewActivity {
    fn from(note_activity: NoteActivity) -> Self {
        let mut activity = NewActivity {
            kind: note_activity.kind.to_string().to_lowercase(),
            uuid: uuid::Uuid::new_v4().to_string(),
            actor: get_ap_id_from_username(note_activity.profile.username.clone()),
            ap_to: serde_json::to_string(&vec![ApAddress::get_public()]).ok(),
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
                    activity.cc = serde_json::to_string(&vec![
                        note.attributed_to,
                        get_followers_ap_id_from_username(note_activity.profile.username),
                    ])
                    .ok();
                } else {
                    activity.cc = serde_json::to_string(&vec![get_followers_ap_id_from_username(
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
                    activity.cc = serde_json::to_string(&vec![
                        remote_note.attributed_to,
                        get_followers_ap_id_from_username(note_activity.profile.username),
                    ])
                    .ok();
                } else {
                    activity.cc = serde_json::to_string(&vec![get_followers_ap_id_from_username(
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
            kind: kind.to_string().to_lowercase(),
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

pub async fn get_activity_by_kind_profile_id_and_target_ap_id(
    conn: &Db,
    kind: ActivityType,
    profile_id: i32,
    target_ap_id: String,
) -> Option<ExtendedActivity> {
    conn.run(move |c| {
        activities::table
            .filter(activities::revoked.eq(false))
            .filter(activities::kind.eq(kind.to_string().to_lowercase()))
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
                    .eq("create".to_string())
                    .or(activities::kind.eq("announce".to_string())),
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
