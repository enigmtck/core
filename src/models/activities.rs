use crate::activity_pub::ApAddress;
use crate::db::Db;
use crate::helper::{
    get_activity_ap_id_from_uuid, get_ap_id_from_username, get_note_ap_id_from_uuid,
};
use crate::schema::{activities, notes, profiles, remote_actors, remote_notes};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::notes::Note;
use super::profiles::{get_profile_by_ap_id, Profile};
use super::remote_actors::RemoteActor;
use super::remote_notes::RemoteNote;

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
}

impl NewActivity {
    pub async fn link(&mut self, conn: &Db) -> Self {
        if let Some(profile) = get_profile_by_ap_id(conn, self.clone().actor).await {
            self.profile_id = Some(profile.id);
        };

        self.clone()
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

        NewActivity {
            kind,
            uuid: uuid::Uuid::new_v4().to_string(),
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
    pub profile_id: i32,
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
