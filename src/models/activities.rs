use crate::activity_pub::{
    ApAcceptType, ApActivity, ApAddress, ApAnnounceType, ApCreateType, ApFollowType, ApLikeType,
    ApUndoType,
};
use crate::db::Db;
use crate::helper::{
    get_activity_ap_id_from_uuid, get_ap_id_from_username, get_followers_ap_id_from_username,
    get_note_ap_id_from_uuid,
};
use crate::routes::inbox::InboxView;
use crate::schema::{activities, activities_cc, activities_to, notes, profiles, remote_actors};
use crate::{MaybeReference, POOL};
use anyhow::anyhow;
use diesel::prelude::*;
use diesel::{AsChangeset, Insertable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Debug};

use super::notes::{Note, NoteLike};
use super::objects::Object;
use super::profiles::{get_profile_by_ap_id, Profile};
use super::remote_actors::RemoteActor;
use super::{from_serde, to_serde};

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use super::pg::activities::ActivityType;

        pub fn to_kind(kind: ActivityType) -> ActivityType {
            kind
        }

        pub use super::pg::activities::NewActivity;
        pub use super::pg::activities::Activity;

        pub use super::pg::activities::ActivityCc;
        pub use super::pg::activities::create_activity_cc;

        pub use super::pg::activities::ActivityTo;
        pub use super::pg::activities::create_activity_to;
        pub use super::pg::activities::create_activity;

        pub use super::pg::activities::get_outbox_activities_by_profile_id;
        pub use super::pg::activities::revoke_activity_by_uuid;
        pub use super::pg::activities::revoke_activity_by_apid;
    } else if #[cfg(feature = "sqlite")] {
        pub use super::sqlite::activities::ActivityType;

        pub fn to_kind(kind: ActivityType) -> String {
            kind.to_string().to_lowercase()
        }

        pub use super::sqlite::activities::NewActivity;
        pub use super::sqlite::activities::Activity;

        pub use super::sqlite::activities::ActivityCc;
        pub use super::sqlite::activities::create_activity_cc;

        pub use super::sqlite::activities::ActivityTo;
        pub use super::sqlite::activities::create_activity_to;
        pub use super::sqlite::activities::create_activity;

        pub use super::sqlite::activities::get_outbox_activities_by_profile_id;
        pub use super::sqlite::activities::revoke_activity_by_uuid;
        pub use super::sqlite::activities::revoke_activity_by_apid;
    }
}

impl fmt::Display for ActivityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl From<String> for ActivityType {
    fn from(activity: String) -> Self {
        match activity.to_lowercase().as_str() {
            "create" => ActivityType::Create,
            "delete" => ActivityType::Delete,
            "update" => ActivityType::Update,
            "announce" => ActivityType::Announce,
            "like" => ActivityType::Like,
            "undo" => ActivityType::Undo,
            "follow" => ActivityType::Follow,
            "accept" => ActivityType::Accept,
            "block" => ActivityType::Block,
            "add" => ActivityType::Add,
            "remove" => ActivityType::Remove,
            _ => ActivityType::Create,
        }
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
    Object(Object),
    Note(Box<Note>),
    Profile(Box<Profile>),
    Activity(Activity),
    RemoteActor(RemoteActor),
}

impl From<Object> for ActivityTarget {
    fn from(object: Object) -> Self {
        ActivityTarget::Object(object)
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

#[derive(Eq, PartialEq, Clone)]
pub enum TimelineView {
    Home(Vec<String>),
    Local,
    Global,
}

impl TryFrom<String> for TimelineView {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(TimelineView::Local),
            "global" => Ok(TimelineView::Global),
            "home" => Ok(TimelineView::Home(vec![])),
            _ => Err(anyhow!("invalid view")),
        }
    }
}

impl From<InboxView> for TimelineView {
    fn from(view: InboxView) -> Self {
        match view {
            InboxView::Local => TimelineView::Local,
            InboxView::Global => TimelineView::Global,
            InboxView::Home => TimelineView::Home(vec![]),
        }
    }
}

#[derive(Clone)]
pub struct TimelineFilters {
    pub view: TimelineView,
    pub hashtags: Vec<String>,
    pub username: Option<String>,
    pub conversation: Option<String>,
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
                ActivityTarget::Object(object) => {
                    self.target_object_id = Some(object.id);
                    self.target_ap_id = Some(object.as_id);
                    self.reply = object.as_in_reply_to.is_some();
                }
                ActivityTarget::Note(note) => {
                    self.target_note_id = Some(note.id);
                    self.target_ap_id = Some(get_note_ap_id_from_uuid(note.uuid));
                    self.reply = note.in_reply_to.is_some();
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
                kind: to_kind(create.kind.into()),
                uuid: uuid.clone(),
                actor: create.actor.to_string(),
                ap_to: to_serde(create.to),
                cc: to_serde(create.cc),
                revoked: false,
                ap_id: create
                    .id
                    .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                ..Default::default()
            }
            .link_target(target)
            .clone()),
            ApActivity::Announce(announce) => Ok(NewActivity {
                kind: to_kind(announce.kind.into()),
                uuid: uuid.clone(),
                actor: announce.actor.to_string(),
                ap_to: to_serde(announce.to),
                cc: to_serde(announce.cc),
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
                kind: to_kind(follow.kind.into()),
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
                        kind: to_kind(accept.kind.into()),
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
                    kind: to_kind(undo.kind.into()),
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
                    kind: to_kind(undo.kind.into()),
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
                    kind: to_kind(undo.kind.into()),
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
                kind: to_kind(like.kind.into()),
                uuid: uuid.clone(),
                actor: like.actor.to_string(),
                target_ap_id: like.object.reference(),
                revoked: false,
                ap_id: like
                    .id
                    .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                ap_to: to_serde(&like.to),
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
                        to_serde(vec![get_ap_id_from_username(profile.username.clone())]).unwrap(),
                    ),
                    Some(get_ap_id_from_username(profile.username)),
                )
            } else if let Some(remote_actor) = remote_actor.clone() {
                (
                    Some(to_serde(vec![remote_actor.ap_id.clone()]).unwrap()),
                    Some(remote_actor.ap_id),
                )
            } else {
                (None, None)
            }
        };

        let uuid = uuid::Uuid::new_v4().to_string();

        NewActivity {
            kind: to_kind(kind),
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
            kind: to_kind(note_activity.kind.clone()),
            uuid: uuid::Uuid::new_v4().to_string(),
            actor: get_ap_id_from_username(note_activity.profile.username.clone()),
            ap_to: to_serde(vec![ApAddress::get_public()]),
            ..Default::default()
        };

        match note_activity.note {
            NoteLike::Note(note) => {
                if note_activity.kind == ActivityType::Like
                    || note_activity.kind == ActivityType::Announce
                {
                    activity.cc = to_serde(vec![
                        note.attributed_to,
                        get_followers_ap_id_from_username(note_activity.profile.username),
                    ]);
                } else {
                    activity.cc = to_serde(vec![get_followers_ap_id_from_username(
                        note_activity.profile.username,
                    )]);
                }
                activity.target_note_id = Some(note.id);
                activity.target_ap_id = Some(get_note_ap_id_from_uuid(note.uuid));
            }
            NoteLike::Object(object) => {
                if note_activity.kind == ActivityType::Like
                    || note_activity.kind == ActivityType::Announce
                {
                    activity.cc = to_serde(vec![
                        from_serde(object.as_attributed_to.unwrap()).unwrap(),
                        get_followers_ap_id_from_username(note_activity.profile.username),
                    ]);
                } else {
                    activity.cc = to_serde(vec![get_followers_ap_id_from_username(
                        note_activity.profile.username,
                    )]);
                }
                activity.target_object_id = Some(object.id);
                activity.target_ap_id = Some(object.as_id);
            }
        }

        activity
    }
}

pub type UndoActivity = (Activity, ActivityType, ApAddress);
impl From<UndoActivity> for NewActivity {
    fn from((activity, kind, actor): UndoActivity) -> Self {
        NewActivity {
            kind: to_kind(kind),
            uuid: uuid::Uuid::new_v4().to_string(),
            actor: actor.to_string(),
            target_activity_id: Some(activity.id),
            target_ap_id: Some(get_activity_ap_id_from_uuid(activity.uuid)),
            revoked: false,
            ..Default::default()
        }
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

#[derive(Serialize, Deserialize, Insertable, AsChangeset, Default, Debug, Clone)]
#[diesel(table_name = activities_to)]
pub struct NewActivityTo {
    pub activity_id: i32,
    pub ap_id: String,
}

pub type ExtendedActivity = (Activity, Option<Note>, Option<Profile>, Option<RemoteActor>);

pub type ExtendedActivityRecord = (Activity, Option<Note>, Option<Profile>, Option<RemoteActor>);

pub async fn get_activity_by_uuid(conn: Option<&Db>, uuid: String) -> Option<ExtendedActivity> {
    let records = match conn {
        Some(conn) => conn
            .run(move |c| {
                activities::table
                        .filter(activities::uuid.eq(uuid))
                        .left_join(
                            notes::table.on(activities::target_note_id.eq(notes::id.nullable())),
                        )
                        .left_join(
                            profiles::table
                                .on(activities::target_profile_id.eq(profiles::id.nullable())),
                        )
                        .left_join(remote_actors::table.on(
                            activities::target_remote_actor_id.eq(remote_actors::id.nullable()),
                        ))
                        .load::<ExtendedActivityRecord>(c)
            })
            .await
            .ok()?,
        None => {
            let mut pool = POOL.get().ok()?;
            activities::table
                .filter(activities::uuid.eq(uuid))
                .left_join(notes::table.on(activities::target_note_id.eq(notes::id.nullable())))
                .left_join(
                    profiles::table.on(activities::target_profile_id.eq(profiles::id.nullable())),
                )
                .left_join(
                    remote_actors::table
                        .on(activities::target_remote_actor_id.eq(remote_actors::id.nullable())),
                )
                .load::<ExtendedActivityRecord>(&mut pool)
                .ok()?
        }
    };

    fold_extended_activity_records(records)
}

pub async fn get_activity_by_apid(conn: &Db, ap_id: String) -> Option<ExtendedActivity> {
    let records = conn
        .run(move |c| {
            activities::table
                .filter(activities::ap_id.eq(ap_id))
                .left_join(notes::table.on(activities::target_note_id.eq(notes::id.nullable())))
                .left_join(
                    profiles::table.on(activities::target_profile_id.eq(profiles::id.nullable())),
                )
                .left_join(
                    remote_actors::table
                        .on(activities::target_remote_actor_id.eq(remote_actors::id.nullable())),
                )
                .load::<ExtendedActivityRecord>(c)
        })
        .await
        .ok()?;

    fold_extended_activity_records(records)
}

pub fn transform_records_to_extended_activities(
    records: Vec<ExtendedActivityRecord>,
) -> Vec<ExtendedActivity> {
    let mut grouped_activities = HashMap::new();

    for (activity_rec, note_rec, profile_rec, remote_actor_rec) in records {
        let activity_id = activity_rec.id;

        let entry = grouped_activities
            .entry(activity_id)
            .or_insert_with(|| (activity_rec, None, None, None));

        // Update the entry with Some values
        if note_rec.is_some() {
            entry.1 = note_rec;
        }
        if profile_rec.is_some() {
            entry.2 = profile_rec;
        }
        if remote_actor_rec.is_some() {
            entry.3 = remote_actor_rec;
        }
    }

    grouped_activities.into_values().collect()
}

pub fn fold_extended_activity_records(
    records: Vec<ExtendedActivityRecord>,
) -> Option<ExtendedActivity> {
    let (activity, note, profile, remote_actor) = records.into_iter().fold(
        (None, None, None, None),
        |(_, _, _, _), (activity_rec, note_rec, profile_rec, remote_actor_rec)| {
            (Some(activity_rec), note_rec, profile_rec, remote_actor_rec)
        },
    );

    Some((activity?, note, profile, remote_actor))
}

pub async fn get_activity_by_kind_profile_id_and_target_ap_id(
    conn: &Db,
    kind: ActivityType,
    profile_id: i32,
    target_ap_id: String,
) -> Option<Activity> {
    conn.run(move |c| {
        activities::table
            .filter(activities::revoked.eq(false))
            .filter(activities::kind.eq(to_kind(kind)))
            .filter(activities::profile_id.eq(profile_id))
            .filter(activities::target_ap_id.eq(target_ap_id))
            .load::<Activity>(c)
    })
    .await
    .ok()?
    .first()
    .cloned()
}

pub async fn get_activity(conn: Option<&Db>, id: i32) -> Option<ExtendedActivity> {
    let records = match conn {
        Some(conn) => conn
            .run(move |c| {
                activities::table
                        .find(id)
                        .left_join(
                            notes::table.on(activities::target_note_id.eq(notes::id.nullable())),
                        )
                        .left_join(
                            profiles::table
                                .on(activities::target_profile_id.eq(profiles::id.nullable())),
                        )
                        .left_join(remote_actors::table.on(
                            activities::target_remote_actor_id.eq(remote_actors::id.nullable()),
                        ))
                        .load::<ExtendedActivityRecord>(c)
            })
            .await
            .ok()?,
        None => {
            let mut pool = POOL.get().ok()?;
            activities::table
                .find(id)
                .left_join(notes::table.on(activities::target_note_id.eq(notes::id.nullable())))
                .left_join(
                    profiles::table.on(activities::target_profile_id.eq(profiles::id.nullable())),
                )
                .left_join(
                    remote_actors::table
                        .on(activities::target_remote_actor_id.eq(remote_actors::id.nullable())),
                )
                .load::<ExtendedActivityRecord>(&mut pool)
                .ok()?
        }
    };

    fold_extended_activity_records(records)
}

pub async fn get_outbox_count_by_profile_id(conn: &Db, profile_id: i32) -> Option<i64> {
    conn.run(move |c| {
        activities::table
            .filter(activities::revoked.eq(false))
            .filter(activities::profile_id.eq(profile_id))
            .filter(
                activities::kind
                    .eq(to_kind(ActivityType::Create))
                    .or(activities::kind.eq(to_kind(ActivityType::Announce))),
            )
            .count()
            .get_result::<i64>(c)
    })
    .await
    .ok()
}

pub async fn update_target_object(
    conn: Option<&Db>,
    activity: Activity,
    object: Object,
) -> Option<usize> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::update(activities::table.find(activity.id))
                    .set(activities::target_object_id.eq(object.id))
                    .execute(c)
                    .ok()
            })
            .await
        }
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::update(activities::table.find(activity.id))
                .set(activities::target_object_id.eq(object.id))
                .execute(&mut pool)
                .ok()
        }
    }
}
