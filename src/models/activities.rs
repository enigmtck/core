use crate::activity_pub::{
    ApAcceptType, ApActivity, ApAddress, ApAnnounceType, ApCreateType, ApFollowType, ApLikeType,
    ApUndoType,
};
use crate::db::Db;
use crate::helper::{
    get_activity_ap_id_from_uuid, get_ap_id_from_username, get_note_ap_id_from_uuid,
};
use crate::schema::{activities, notes, profiles, remote_actors, remote_notes};
use crate::MaybeReference;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::notes::Note;
use super::profiles::{get_profile_by_ap_id, Profile};
use super::remote_actors::RemoteActor;
use super::remote_notes::RemoteNote;

#[derive(
    diesel_derive_enum::DbEnum,
    Debug,
    Serialize,
    Deserialize,
    Default,
    Clone,
    Eq,
    PartialEq,
    QueryId,
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
    Note(Note),
    RemoteNote(RemoteNote),
    Profile(Profile),
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
        ActivityTarget::Profile(profile)
    }
}

impl From<Activity> for ActivityTarget {
    fn from(activity: Activity) -> Self {
        ActivityTarget::Activity(activity)
    }
}

impl From<Note> for ActivityTarget {
    fn from(note: Note) -> Self {
        ActivityTarget::Note(note)
    }
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

impl NewActivity {
    pub async fn link(&mut self, conn: &Db) -> Self {
        if let Some(profile) = get_profile_by_ap_id(conn, self.clone().actor).await {
            self.profile_id = Some(profile.id);
        };

        self.clone()
    }

    pub fn link_target(&mut self, target: ActivityTarget) -> &Self {
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
        self
    }
}

impl TryFrom<ApActivity> for NewActivity {
    type Error = &'static str;

    // eventually I may be able to move decomposition logic here (e.g., create target_remote_note, etc.)
    // that will require the ability to `await` on database calls
    //
    // https://blog.rust-lang.org/inside-rust/2023/05/03/stabilizing-async-fn-in-trait.html
    fn try_from(activity: ApActivity) -> Result<Self, Self::Error> {
        let uuid = uuid::Uuid::new_v4().to_string();

        match activity {
            ApActivity::Create(create) => Ok(NewActivity {
                kind: create.kind.into(),
                uuid: uuid.clone(),
                actor: create.actor.to_string(),
                ap_to: serde_json::to_value(create.to).ok(),
                cc: serde_json::to_value(create.cc).ok(),
                profile_id: None,
                target_note_id: None,
                target_remote_note_id: None,
                target_profile_id: None,
                target_activity_id: None,
                target_ap_id: None,
                target_remote_actor_id: None,
                revoked: false,
                ap_id: create
                    .id
                    .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
            }),
            ApActivity::Announce(announce) => Ok(NewActivity {
                kind: announce.kind.into(),
                uuid: uuid.clone(),
                actor: announce.actor.to_string(),
                ap_to: serde_json::to_value(announce.to).ok(),
                cc: serde_json::to_value(announce.cc).ok(),
                profile_id: None,
                target_note_id: None,
                target_remote_note_id: None,
                target_profile_id: None,
                target_activity_id: None,
                target_ap_id: announce.object.reference(),
                target_remote_actor_id: None,
                revoked: false,
                ap_id: announce
                    .id
                    .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
            }),
            ApActivity::Follow(follow) => Ok(NewActivity {
                kind: follow.kind.into(),
                uuid: uuid.clone(),
                actor: follow.actor.to_string(),
                ap_to: None,
                cc: None,
                profile_id: None,
                target_note_id: None,
                target_remote_note_id: None,
                target_profile_id: None,
                target_activity_id: None,
                target_ap_id: follow.object.reference(),
                target_remote_actor_id: None,
                revoked: false,
                ap_id: follow
                    .id
                    .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
            }),
            ApActivity::Accept(accept) => {
                if let MaybeReference::Actual(ApActivity::Follow(follow)) = accept.object {
                    Ok(NewActivity {
                        kind: accept.kind.into(),
                        uuid: uuid.clone(),
                        actor: accept.actor.to_string(),
                        ap_to: None,
                        cc: None,
                        profile_id: None,
                        target_note_id: None,
                        target_remote_note_id: None,
                        target_profile_id: None,
                        target_activity_id: None,
                        target_ap_id: follow.id,
                        target_remote_actor_id: None,
                        revoked: false,
                        ap_id: accept
                            .id
                            .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                    })
                } else {
                    Err("ACCEPT OBJECT NOT AN ACTUAL")
                }
            }
            ApActivity::Undo(undo) => {
                if let MaybeReference::Actual(ApActivity::Follow(follow)) = undo.object {
                    Ok(NewActivity {
                        kind: undo.kind.into(),
                        uuid: uuid.clone(),
                        actor: undo.actor.to_string(),
                        ap_to: None,
                        cc: None,
                        profile_id: None,
                        target_note_id: None,
                        target_remote_note_id: None,
                        target_profile_id: None,
                        target_activity_id: None,
                        target_ap_id: follow.id,
                        target_remote_actor_id: None,
                        revoked: false,
                        ap_id: undo
                            .id
                            .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                    })
                } else {
                    Err("UNDO OBJECT NOT AN ACTUAL")
                }
            }
            ApActivity::Like(like) => Ok(NewActivity {
                kind: like.kind.into(),
                uuid: uuid.clone(),
                actor: like.actor.to_string(),
                ap_to: None,
                cc: None,
                profile_id: None,
                target_note_id: None,
                target_remote_note_id: None,
                target_profile_id: None,
                target_activity_id: None,
                target_ap_id: like.object.reference(),
                target_remote_actor_id: None,
                revoked: false,
                ap_id: like
                    .id
                    .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
            }),

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
                        serde_json::to_value(vec![get_ap_id_from_username(
                            profile.username.clone(),
                        )])
                        .unwrap(),
                    ),
                    Some(get_ap_id_from_username(profile.username)),
                )
            } else if let Some(remote_actor) = remote_actor.clone() {
                (
                    Some(serde_json::to_value(vec![remote_actor.ap_id.clone()]).unwrap()),
                    Some(remote_actor.ap_id),
                )
            } else {
                (None, None)
            }
        };

        let uuid = uuid::Uuid::new_v4().to_string();

        NewActivity {
            kind,
            uuid: uuid.clone(),
            actor: actor.to_string(),
            ap_to,
            cc: None,
            profile_id: None,
            target_note_id: None,
            target_remote_note_id: None,
            target_profile_id: profile.map(|x| x.id),
            target_activity_id: None,
            target_ap_id,
            target_remote_actor_id: remote_actor.map(|x| x.id),
            revoked: false,
            ap_id: Some(get_activity_ap_id_from_uuid(uuid)),
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
                    Some(serde_json::to_value(vec![note.attributed_to]).unwrap()),
                    Some(get_note_ap_id_from_uuid(note.uuid)),
                )
            } else if let Some(remote_note) = remote_note.clone() {
                (
                    remote_note.ap_to,
                    Some(serde_json::to_value(vec![remote_note.attributed_to]).unwrap()),
                    Some(remote_note.ap_id),
                )
            } else {
                (None, None, None)
            }
        };

        NewActivity {
            kind,
            uuid: uuid::Uuid::new_v4().to_string(),
            actor: actor.to_string(),
            ap_to,
            cc,
            profile_id: None,
            target_note_id: note.map(|x| x.id),
            target_remote_note_id: remote_note.map(|x| x.id),
            target_profile_id: None,
            target_activity_id: None,
            target_ap_id,
            target_remote_actor_id: None,
            revoked: false,
            ap_id: None,
        }
    }
}

pub type UndoActivity = (Activity, ActivityType, ApAddress);
impl From<UndoActivity> for NewActivity {
    fn from((activity, kind, actor): UndoActivity) -> Self {
        NewActivity {
            kind,
            uuid: uuid::Uuid::new_v4().to_string(),
            actor: actor.to_string(),
            ap_to: None,
            cc: None,
            profile_id: None,
            target_note_id: None,
            target_remote_note_id: None,
            target_profile_id: None,
            target_activity_id: Some(activity.id),
            target_ap_id: Some(get_activity_ap_id_from_uuid(activity.uuid)),
            target_remote_actor_id: None,
            revoked: false,
            ap_id: None,
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

pub async fn create_activity(conn: &Db, activity: NewActivity) -> Option<Activity> {
    conn.run(move |c| {
        diesel::insert_into(activities::table)
            .values(&activity)
            .get_result::<Activity>(c)
    })
    .await
    .ok()
}

pub type ExtendedActivity = (
    Activity,
    Option<Note>,
    Option<RemoteNote>,
    Option<Profile>,
    Option<RemoteActor>,
);

pub async fn get_activity_by_uuid(conn: &Db, uuid: String) -> Option<ExtendedActivity> {
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

pub async fn get_activity(conn: &Db, id: i32) -> Option<ExtendedActivity> {
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
            let date: DateTime<Utc> =
                DateTime::from_utc(NaiveDateTime::from_timestamp_micros(min).unwrap(), Utc);

            log::debug!("MINIMUM {date:#?}");

            query = query
                .filter(activities::created_at.gt(date))
                .order(activities::created_at.asc());
        } else if let Some(max) = max {
            let date: DateTime<Utc> =
                DateTime::from_utc(NaiveDateTime::from_timestamp_micros(max).unwrap(), Utc);

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
