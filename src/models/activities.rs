use crate::activity_pub::{
    ApAcceptType, ApAnnounceType, ApCreateType, ApFollowType, ApLikeType, ApUndoType,
};
use crate::db::Db;
use crate::helper::{
    get_activity_ap_id_from_uuid, get_ap_id_from_username, get_note_ap_id_from_uuid,
};
use crate::schema::{
    activities, activities_cc, activities_to, notes, profiles, remote_actors, remote_notes,
};
use crate::POOL;
use diesel::prelude::*;
use diesel::{AsChangeset, Insertable};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};

use super::notes::Note;
use super::profiles::{get_profile_by_ap_id, Profile};
use super::remote_actors::RemoteActor;
use super::remote_notes::RemoteNote;
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use super::pg::activities::ActivityType;
    } else if #[cfg(feature = "sqlite")] {
        pub use super::sqlite::activities::ActivityType;
    }
}

impl fmt::Display for ActivityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
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

cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use super::pg::activities::NewActivity;
    } else if #[cfg(feature = "sqlite")] {
        pub use super::sqlite::activities::NewActivity;
    }
}

impl NewActivity {
    pub async fn link_profile(&mut self, conn: &Db) -> Self {
        if let Some(profile) = get_profile_by_ap_id(Some(conn), self.clone().actor).await {
            self.profile_id = Some(profile.id);
        };

        self.clone()
    }

    pub fn link_target(&mut self, target: Option<ActivityTarget>) -> &Self {
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

cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use super::pg::activities::ApActivityTarget;
    } else if #[cfg(feature = "sqlite")] {
        pub use super::sqlite::activities::ApActivityTarget;
    }
}

cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use super::pg::activities::ActorActivity;
    } else if #[cfg(feature = "sqlite")] {
        pub use super::sqlite::activities::ActorActivity;
    }
}

cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use super::pg::activities::NoteActivity;
        pub use super::pg::activities::UndoActivity;
    } else if #[cfg(feature = "sqlite")] {
        pub use super::sqlite::activities::NoteActivity;
        pub use super::sqlite::activities::UndoActivity;
    }
}

cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use super::pg::activities::Activity;
    } else if #[cfg(feature = "sqlite")] {
        pub use super::sqlite::activities::Activity;
    }
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

cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use super::pg::activities::ActivityCc;
        pub use super::pg::activities::create_activity_cc;
    } else if #[cfg(feature = "sqlite")] {
        pub use super::sqlite::activities::ActivityCc;
        pub use super::sqlite::activities::create_activity_cc;
    }
}

#[derive(Serialize, Deserialize, Insertable, AsChangeset, Default, Debug, Clone)]
#[diesel(table_name = activities_to)]
pub struct NewActivityTo {
    pub activity_id: i32,
    pub ap_id: String,
}

cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use super::pg::activities::ActivityTo;
        pub use super::pg::activities::create_activity_to;
        pub use super::pg::activities::create_activity;
    } else if #[cfg(feature = "sqlite")] {
        pub use super::sqlite::activities::ActivityTo;
        pub use super::sqlite::activities::create_activity_to;
        pub use super::sqlite::activities::create_activity;
    }
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

cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use super::pg::activities::get_activity_by_kind_profile_id_and_target_ap_id;
        pub use super::pg::activities::get_outbox_count_by_profile_id;
        pub use super::pg::activities::get_outbox_activities_by_profile_id;
        pub use super::pg::activities::revoke_activity_by_uuid;
        pub use super::pg::activities::revoke_activity_by_apid;
    } else if #[cfg(feature = "sqlite")] {
        pub use super::sqlite::activities::get_activity_by_kind_profile_id_and_target_ap_id;
        pub use super::sqlite::activities::get_outbox_count_by_profile_id;
        pub use super::sqlite::activities::get_outbox_activities_by_profile_id;
        pub use super::sqlite::activities::revoke_activity_by_uuid;
        pub use super::sqlite::activities::revoke_activity_by_apid;
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
