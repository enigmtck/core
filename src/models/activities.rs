use crate::activity_pub::{
    ApAcceptType, ApActivity, ApAddress, ApAnnounceType, ApCreateType, ApFollowType, ApLikeType,
    ApUndoType,
};
use crate::db::Db;
use crate::helper::{
    get_activity_ap_id_from_uuid, get_ap_id_from_username, get_note_ap_id_from_uuid,
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
use std::fmt::{self, Debug};

use super::notes::Note;
use super::profiles::{get_profile_by_ap_id, Profile};
use super::remote_actors::RemoteActor;
use super::remote_notes::RemoteNote;

#[derive(Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
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

impl fmt::Display for ActivityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl From<ActivityType> for String {
    fn from(t: ActivityType) -> String {
        format!("{t}")
    }
}

impl From<ApCreateType> for ActivityType {
    fn from(_: ApCreateType) -> Self {
        ActivityType::Create
    }
}

impl From<ApAnnounceType> for ActivityType {
    fn from(_: ApAnnounceType) -> Self {
        ActivityType::Announce
    }
}

impl From<ApFollowType> for ActivityType {
    fn from(_: ApFollowType) -> Self {
        ActivityType::Follow
    }
}

impl From<ApAcceptType> for ActivityType {
    fn from(_: ApAcceptType) -> Self {
        ActivityType::Accept
    }
}

impl From<ApUndoType> for ActivityType {
    fn from(_: ApUndoType) -> Self {
        ActivityType::Undo
    }
}

impl From<ApLikeType> for ActivityType {
    fn from(_: ApLikeType) -> Self {
        ActivityType::Like
    }
}

pub enum ActivityTarget {
    Note(Box<Note>),
    RemoteNote(RemoteNote),
    Profile(Box<Profile>),
    Activity(Activity),
    RemoteActor(RemoteActor),
}

impl From<RemoteNote> for ActivityTarget {
    fn from(remote_note: RemoteNote) -> Self {
        ActivityTarget::RemoteNote(remote_note)
    }
}

impl From<Profile> for ActivityTarget {
    fn from(profile: Profile) -> Self {
        ActivityTarget::Profile(Box::new(profile))
    }
}

impl From<Activity> for ActivityTarget {
    fn from(activity: Activity) -> Self {
        ActivityTarget::Activity(activity)
    }
}

impl From<Note> for ActivityTarget {
    fn from(note: Note) -> Self {
        ActivityTarget::Note(Box::new(note))
    }
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

impl NewActivity {
    pub async fn link_profile(&mut self, conn: &Db) -> Self {
        if let Some(profile) = get_profile_by_ap_id(Some(conn), self.clone().actor).await {
            self.profile_id = Some(profile.id);
        };

        self.clone()
    }

    fn link_target(&mut self, target: Option<ActivityTarget>) -> &Self {
        if let Some(target) = target {
            match target {
                ActivityTarget::Note(note) => {
                    self.target_note_id = Some(note.id);
                    self.target_ap_id = Some(get_note_ap_id_from_uuid(note.uuid));
                }
                ActivityTarget::RemoteNote(remote_note) => {
                    self.target_remote_note_id = Some(remote_note.id);
                    self.target_ap_id = Some(remote_note.ap_id);
                }
                ActivityTarget::Profile(profile) => {
                    self.target_profile_id = Some(profile.id);
                    self.target_ap_id = Some(get_ap_id_from_username(profile.username));
                }
                ActivityTarget::Activity(activity) => {
                    self.target_activity_id = Some(activity.id);
                    self.target_ap_id = activity
                        .ap_id
                        .map_or(Some(get_activity_ap_id_from_uuid(activity.uuid)), Some);
                }
                ActivityTarget::RemoteActor(remote_actor) => {
                    self.target_remote_actor_id = Some(remote_actor.id);
                    self.target_ap_id = Some(remote_actor.ap_id);
                }
            }
        };

        self
    }
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
                kind: create.kind.into(),
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
                kind: announce.kind.into(),
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
                kind: follow.kind.into(),
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
                        kind: accept.kind.into(),
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
                    kind: undo.kind.into(),
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
                    kind: undo.kind.into(),
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
                    kind: undo.kind.into(),
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
                kind: like.kind.into(),
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
            kind: String::from(kind),
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

pub type NoteActivity = (Option<Note>, Option<RemoteNote>, ActivityType, ApAddress);
impl From<NoteActivity> for NewActivity {
    fn from((note, remote_note, kind, actor): NoteActivity) -> Self {
        let (ap_to, cc, target_ap_id) = {
            if let Some(note) = note.clone() {
                (
                    Some(note.ap_to),
                    note.cc,
                    Some(get_note_ap_id_from_uuid(note.uuid)),
                )
            } else if let Some(remote_note) = remote_note.clone() {
                (remote_note.ap_to, remote_note.cc, Some(remote_note.ap_id))
            } else {
                (None, None, None)
            }
        };

        NewActivity {
            kind: String::from(kind),
            uuid: uuid::Uuid::new_v4().to_string(),
            actor: actor.to_string(),
            ap_to,
            cc,
            target_note_id: note.map(|x| x.id),
            target_remote_note_id: remote_note.map(|x| x.id),
            target_ap_id,
            revoked: false,
            ..Default::default()
        }
    }
}

pub type UndoActivity = (Activity, ActivityType, ApAddress);
impl From<UndoActivity> for NewActivity {
    fn from((activity, kind, actor): UndoActivity) -> Self {
        NewActivity {
            kind: kind.into(),
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

type AssignedActivity = (Activity, String);

impl From<AssignedActivity> for NewActivityCc {
    fn from(assigned_activity: AssignedActivity) -> Self {
        NewActivityCc {
            activity_id: assigned_activity.0.id,
            ap_id: assigned_activity.1,
        }
    }
}

impl From<AssignedActivity> for NewActivityTo {
    fn from(assigned_activity: AssignedActivity) -> Self {
        NewActivityTo {
            activity_id: assigned_activity.0.id,
            ap_id: assigned_activity.1,
        }
    }
}

#[derive(Serialize, Deserialize, Insertable, AsChangeset, Default, Debug, Clone)]
#[diesel(table_name = activities_cc)]
pub struct NewActivityCc {
    pub activity_id: i32,
    pub ap_id: String,
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

async fn create_activity_cc(conn: Option<&Db>, activity_cc: NewActivityCc) -> bool {
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

#[derive(Serialize, Deserialize, Insertable, AsChangeset, Default, Debug, Clone)]
#[diesel(table_name = activities_to)]
pub struct NewActivityTo {
    pub activity_id: i32,
    pub ap_id: String,
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

async fn create_activity_to(conn: Option<&Db>, activity_to: NewActivityTo) -> bool {
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

pub type ExtendedActivity = (
    Activity,
    Option<Note>,
    Option<RemoteNote>,
    Option<Profile>,
    Option<RemoteActor>,
);

pub async fn get_activity_by_uuid(conn: Option<&Db>, uuid: String) -> Option<ExtendedActivity> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                activities::table
                    .filter(activities::uuid.eq(uuid))
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
        None => {
            let mut pool = POOL.get().ok()?;
            activities::table
                .filter(activities::uuid.eq(uuid))
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
                .first::<ExtendedActivity>(&mut pool)
                .ok()
        }
    }
}

pub async fn get_activity_by_apid(conn: &Db, ap_id: String) -> Option<ExtendedActivity> {
    conn.run(move |c| {
        activities::table
            .filter(activities::ap_id.eq(ap_id))
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

pub async fn get_activity(conn: Option<&Db>, id: i32) -> Option<ExtendedActivity> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                activities::table
                    .find(id)
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
        None => {
            let mut pool = POOL.get().ok()?;
            activities::table
                .find(id)
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
                .first::<ExtendedActivity>(&mut pool)
                .ok()
        }
    }
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
            .filter(activities::kind.eq(String::from(kind)))
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

pub async fn update_target_remote_note(
    conn: Option<&Db>,
    activity: Activity,
    remote_note: RemoteNote,
) -> Option<usize> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::update(activities::table.find(activity.id))
                    .set(activities::target_remote_note_id.eq(remote_note.id))
                    .execute(c)
                    .ok()
            })
            .await
        }
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::update(activities::table.find(activity.id))
                .set(activities::target_remote_note_id.eq(remote_note.id))
                .execute(&mut pool)
                .ok()
        }
    }
}
