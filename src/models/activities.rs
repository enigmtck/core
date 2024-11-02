use crate::activity_pub::{
    ApAcceptType, ApActivity, ApAddress, ApAnnounceType, ApCreateType, ApDeleteType, ApFollowType,
    ApLikeType, ApUndoType,
};
use crate::db::Db;
use crate::helper::get_activity_ap_id_from_uuid;
use crate::routes::inbox::InboxView;
use crate::schema::activities;
use crate::{MaybeReference, POOL};
use anyhow::anyhow;
use diesel::prelude::*;
use std::fmt::{self, Debug};

use super::actors::{get_actor_by_as_id, Actor};
use super::objects::Object;
use super::pg::activities::get_activities_coalesced;
use super::pg::coalesced_activity::CoalescedActivity;
use super::to_serde;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use super::pg::activities::ActivityType;

        pub fn to_kind(kind: ActivityType) -> ActivityType {
            kind
        }

        pub use super::pg::activities::NewActivity;
        pub use super::pg::activities::Activity;

        pub use super::pg::activities::create_activity;

        pub use super::pg::activities::revoke_activity_by_uuid;
        pub use super::pg::activities::revoke_activity_by_apid;
        pub use super::pg::activities::revoke_activities_by_object_as_id;
        pub use super::pg::activities::add_log_by_as_id;
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

impl From<ApDeleteType> for ActivityType {
    fn from(_: ApDeleteType) -> Self {
        ActivityType::Delete
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
    Activity(Activity),
    Actor(Actor),
}

impl From<Object> for ActivityTarget {
    fn from(object: Object) -> Self {
        ActivityTarget::Object(object)
    }
}

impl From<Actor> for ActivityTarget {
    fn from(actor: Actor) -> Self {
        ActivityTarget::Actor(actor)
    }
}

impl From<Activity> for ActivityTarget {
    fn from(activity: Activity) -> Self {
        ActivityTarget::Activity(activity)
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
    pub view: Option<TimelineView>,
    pub hashtags: Vec<String>,
    pub username: Option<String>,
    pub conversation: Option<String>,
}

impl NewActivity {
    pub async fn link_actor(&mut self, conn: &Db) -> Self {
        if let Ok(actor) = get_actor_by_as_id(conn, self.clone().actor).await {
            self.actor_id = Some(actor.id);
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
                ActivityTarget::Activity(activity) => {
                    self.target_activity_id = Some(activity.id);
                    self.target_ap_id = activity.ap_id;
                    self.reply = false;
                }
                ActivityTarget::Actor(actor) => {
                    self.target_actor_id = Some(actor.id);
                    self.target_ap_id = Some(actor.as_id);
                    self.reply = false;
                }
            }
        };

        self
    }
}

pub type ApActivityTarget = (ApActivity, Option<ActivityTarget>);

impl TryFrom<ApActivityTarget> for NewActivity {
    type Error = anyhow::Error;

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
                ap_to: to_serde(&Some(create.to)),
                cc: to_serde(&create.cc),
                revoked: false,
                ap_id: create
                    .id
                    .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                ..Default::default()
            }
            .link_target(target)
            .clone()),
            ApActivity::Delete(delete) => Ok(NewActivity {
                kind: to_kind(delete.kind.into()),
                uuid: uuid.clone(),
                actor: delete.actor.to_string(),
                ap_to: to_serde(&Some(delete.to)),
                cc: to_serde(&delete.cc),
                target_ap_id: delete.object.reference(),
                revoked: false,
                ap_id: delete
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
                ap_to: to_serde(&Some(announce.to)),
                cc: to_serde(&announce.cc),
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
                target_actor_id: None,
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
                    Err(anyhow!("ACCEPT OBJECT NOT AN ACTUAL"))
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
                _ => Err(anyhow!("UNDO OBJECT NOT IMPLEMENTED")),
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
                ap_to: to_serde(&Some(&like.to)),
                ..Default::default()
            }
            .link_target(target)
            .clone()),

            _ => Err(anyhow!("UNIMPLEMENTED ACTIVITY TYPE")),
        }
    }
}

pub type ActorActivity = (Option<Actor>, Option<Actor>, ActivityType, ApAddress);

impl From<ActorActivity> for NewActivity {
    fn from((profile, remote_actor, kind, actor): ActorActivity) -> Self {
        let (ap_to, target_ap_id) = {
            if let Some(profile) = profile.clone() {
                (
                    Some(to_serde(&Some(vec![profile.as_id.clone()])).unwrap()),
                    Some(profile.as_id),
                )
            } else if let Some(remote_actor) = remote_actor.clone() {
                (
                    Some(to_serde(&Some(vec![remote_actor.as_id.clone()])).unwrap()),
                    Some(remote_actor.as_id),
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
            target_actor_id: remote_actor.map(|x| x.id),
            target_ap_id,
            revoked: false,
            ap_id: Some(get_activity_ap_id_from_uuid(uuid)),
            ..Default::default()
        }
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

pub type ExtendedActivity = (Activity, Option<Activity>, Option<Object>, Option<Actor>);

fn target_to_main(coalesced: CoalescedActivity) -> Option<Activity> {
    Some(Activity {
        id: coalesced.target_activity_id?,
        created_at: coalesced.recursive_created_at?,
        updated_at: coalesced.recursive_updated_at?,
        kind: coalesced.recursive_kind?,
        uuid: coalesced.recursive_uuid?,
        actor: coalesced.recursive_actor?,
        ap_to: coalesced.recursive_ap_to,
        cc: coalesced.recursive_cc,
        target_activity_id: coalesced.recursive_target_activity_id,
        target_ap_id: coalesced.recursive_target_ap_id,
        revoked: coalesced.recursive_revoked?,
        ap_id: coalesced.recursive_ap_id,
        reply: coalesced.recursive_reply?,
        raw: None,
        target_object_id: coalesced.recursive_target_object_id,
        actor_id: coalesced.recursive_actor_id,
        target_actor_id: coalesced.recursive_target_actor_id,
        log: None,
    })
}

impl From<CoalescedActivity> for ExtendedActivity {
    fn from(coalesced: CoalescedActivity) -> ExtendedActivity {
        let activity = Activity {
            id: coalesced.id,
            created_at: coalesced.created_at,
            updated_at: coalesced.updated_at,
            kind: coalesced.kind.clone(),
            uuid: coalesced.uuid.clone(),
            actor: coalesced.actor.clone(),
            ap_to: coalesced.ap_to.clone(),
            cc: coalesced.cc.clone(),
            target_activity_id: coalesced.target_activity_id,
            target_ap_id: coalesced.target_ap_id.clone(),
            revoked: coalesced.revoked,
            ap_id: coalesced.ap_id.clone(),
            reply: coalesced.reply,
            raw: coalesced.raw.clone(),
            target_object_id: coalesced.target_object_id,
            actor_id: coalesced.actor_id,
            target_actor_id: coalesced.target_actor_id,
            log: coalesced.log.clone(),
        };

        let target_activity = target_to_main(coalesced.clone());
        let target_object: Option<Object> = coalesced.clone().try_into().ok();
        let target_actor: Option<Actor> = coalesced.try_into().ok();

        (activity, target_activity, target_object, target_actor)
    }
}

pub async fn get_activity_by_ap_id(conn: &Db, ap_id: String) -> Option<ExtendedActivity> {
    get_activities_coalesced(conn, 1, None, None, None, None, Some(ap_id), None, None)
        .await
        .first()
        .cloned()
        .map(ExtendedActivity::from)
}

pub async fn get_activity_by_kind_actor_id_and_target_ap_id(
    conn: &Db,
    kind: ActivityType,
    actor_id: i32,
    target_ap_id: String,
) -> Option<Activity> {
    conn.run(move |c| {
        activities::table
            .filter(activities::revoked.eq(false))
            .filter(activities::kind.eq(to_kind(kind)))
            .filter(activities::actor_id.eq(actor_id))
            .filter(activities::target_ap_id.eq(target_ap_id))
            .load::<Activity>(c)
    })
    .await
    .ok()?
    .first()
    .cloned()
}

pub async fn get_activity(conn: Option<&Db>, id: i32) -> Option<ExtendedActivity> {
    get_activities_coalesced(
        conn.unwrap(),
        1,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(id),
    )
    .await
    .first()
    .cloned()
    .map(ExtendedActivity::from)
}

pub async fn get_outbox_count_by_actor_id(conn: &Db, actor_id: i32) -> Option<i64> {
    conn.run(move |c| {
        activities::table
            .filter(activities::revoked.eq(false))
            .filter(activities::actor_id.eq(actor_id))
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
