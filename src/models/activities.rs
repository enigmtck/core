use crate::db::runner::DbRunner;
use crate::db::DbType;
use crate::helper::get_activity_ap_id_from_uuid;
use crate::models::actors::{get_actor_by_as_id, Actor};
use crate::models::coalesced_activity::CoalescedActivity;
use crate::models::objects::Object;
use crate::models::parameter_generator;
use crate::schema::{activities, actors};
use crate::server::InboxView;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use convert_case::{Case, Casing};
use diesel::query_builder::{BoxedSqlQuery, SqlQuery};
use diesel::sql_types::Nullable;
use diesel::{prelude::*, sql_query};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use indoc::indoc;
use jdt_activity_pub::ApBlockType;
use jdt_activity_pub::ApMoveType;
use jdt_activity_pub::ApRemoveType;
use jdt_activity_pub::MaybeMultiple;
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{
    ApAccept, ApAcceptType, ApActivity, ApAddress, ApAnnounce, ApAnnounceType, ApContext, ApCreate,
    ApCreateType, ApDelete, ApDeleteType, ApFollow, ApFollowType, ApInstrument, ApLike, ApLikeType,
    ApNote, ApObject, ApUndo, ApUndoType, ApUpdateType, Ephemeral,
};
use jdt_activity_pub::{ApUpdate, PUBLIC_COLLECTION};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt::{self, Debug};

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
    Move,
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

    pub fn is_block(&self) -> bool {
        self == &ActivityType::Block
    }

    pub fn is_add(&self) -> bool {
        self == &ActivityType::Add
    }

    pub fn is_remove(&self) -> bool {
        self == &ActivityType::Remove
    }

    pub fn is_move(&self) -> bool {
        self == &ActivityType::Move
    }
}

impl fmt::Display for ActivityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl From<ActivityType> for String {
    fn from(activity: ActivityType) -> Self {
        format!("{activity}").to_case(Case::Snake)
    }
}

impl TryFrom<ActivityType> for ApCreateType {
    type Error = anyhow::Error;

    fn try_from(t: ActivityType) -> Result<Self, Self::Error> {
        match t {
            ActivityType::Create => Ok(ApCreateType::Create),
            _ => Err(anyhow!("invalid ActivityType")),
        }
    }
}

impl TryFrom<ActivityType> for ApAnnounceType {
    type Error = anyhow::Error;

    fn try_from(t: ActivityType) -> Result<Self, Self::Error> {
        match t {
            ActivityType::Announce => Ok(ApAnnounceType::Announce),
            _ => Err(anyhow!("invalid ActivityType")),
        }
    }
}

impl TryFrom<String> for ActivityType {
    type Error = anyhow::Error;

    fn try_from(activity: String) -> Result<Self> {
        match activity.to_case(Case::Snake).as_str() {
            "create" => Ok(ActivityType::Create),
            "delete" => Ok(ActivityType::Delete),
            "update" => Ok(ActivityType::Update),
            "announce" => Ok(ActivityType::Announce),
            "like" => Ok(ActivityType::Like),
            "undo" => Ok(ActivityType::Undo),
            "follow" => Ok(ActivityType::Follow),
            "accept" => Ok(ActivityType::Accept),
            "block" => Ok(ActivityType::Block),
            "add" => Ok(ActivityType::Add),
            "remove" => Ok(ActivityType::Remove),
            "move" => Ok(ActivityType::Move),
            _ => Err(anyhow!("unimplemented ActivityType")),
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

impl From<ApUpdateType> for ActivityType {
    fn from(_: ApUpdateType) -> Self {
        ActivityType::Update
    }
}

impl From<ApLikeType> for ActivityType {
    fn from(_: ApLikeType) -> Self {
        ActivityType::Like
    }
}

impl From<ApBlockType> for ActivityType {
    fn from(_: ApBlockType) -> Self {
        ActivityType::Block
    }
}

impl From<ApRemoveType> for ActivityType {
    fn from(_: ApRemoveType) -> Self {
        ActivityType::Remove
    }
}

impl From<ApMoveType> for ActivityType {
    fn from(_: ApMoveType) -> Self {
        ActivityType::Move
    }
}

#[derive(Clone)]
pub enum ActivityTarget {
    Object(Object),
    Activity(Activity),
    Actor(Actor),
}

impl From<&Object> for ActivityTarget {
    fn from(object: &Object) -> Self {
        ActivityTarget::Object(object.clone())
    }
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
    Direct,
}

impl TryFrom<String> for TimelineView {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(TimelineView::Local),
            "global" => Ok(TimelineView::Global),
            "home" => Ok(TimelineView::Home(vec![])),
            "direct" => Ok(TimelineView::Direct),
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
            InboxView::Direct => TimelineView::Direct,
        }
    }
}

#[derive(Clone)]
pub struct TimelineFilters {
    pub view: Option<TimelineView>,
    pub hashtags: Vec<String>,
    pub username: Option<String>,
    pub conversation: Option<String>,
    pub excluded_words: Vec<String>,
    pub direct: bool,
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

    #[cfg(feature = "pg")]
    pub kind: ActivityType,

    #[cfg(feature = "sqlite")]
    pub kind: String,

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
    // pub async fn extend(&self, conn: &Db) -> Option<ExtendedActivity> {
    //     get_activity(Some(conn), self.id).await
    // }

    pub async fn extend<C: DbRunner>(&self, conn: &C) -> Option<ExtendedActivity> {
        get_activity(conn, self.id).await.ok().flatten()
    }
}

#[derive(Serialize, Deserialize, Insertable, Debug, Clone, AsChangeset)]
#[diesel(table_name = activities)]
pub struct NewActivity {
    #[cfg(feature = "pg")]
    pub kind: ActivityType,

    #[cfg(feature = "sqlite")]
    pub kind: String,

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

impl NewActivity {
    pub async fn link_actor<C: DbRunner>(&mut self, conn: &C) -> Self {
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

    pub fn set_raw(&mut self, raw: Value) -> Self {
        self.raw = Some(raw);

        self.clone()
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
                kind: create.kind.into(),
                uuid: uuid.clone(),
                actor: create.actor.to_string(),
                ap_to: create.to.into(),
                cc: create.cc.into(),
                revoked: false,
                ap_id: create
                    .id
                    .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                instrument: (&create.instrument).into(),
                ..Default::default()
            }
            .link_target(target)
            .clone()),
            ApActivity::Delete(delete) => Ok(NewActivity {
                kind: delete.kind.into(),
                uuid: uuid.clone(),
                actor: delete.actor.to_string(),
                ap_to: delete.to.into(),
                cc: delete.cc.into(),
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
                kind: announce.kind.into(),
                uuid: uuid.clone(),
                actor: announce.actor.to_string(),
                ap_to: announce.to.into(),
                cc: announce.cc.into(),
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
                if let Some(ActivityTarget::Activity(follow)) = target.clone() {
                    Ok(NewActivity {
                        kind: accept.kind.into(),
                        uuid: uuid.clone(),
                        actor: accept.actor.to_string(),
                        target_ap_id: follow.ap_id,
                        revoked: false,
                        ap_id: accept
                            .id
                            .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                        ..Default::default()
                    }
                    .link_target(target)
                    .clone())
                } else {
                    Err(anyhow!(
                        "Unable to locate Follow in TryFrom<ApActivityTarget> for NewActivity"
                    ))
                }
            }
            ApActivity::Update(update) => match update.object.clone() {
                MaybeReference::Actual(ApObject::Actor(actor)) => Ok(NewActivity {
                    kind: update.kind.clone().into(),
                    uuid: uuid.clone(),
                    actor: update.actor.to_string(),
                    target_ap_id: actor.id.map(|x| x.to_string()),
                    revoked: false,
                    ap_id: update
                        .id
                        .clone()
                        .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                    raw: Some(json!(update)),
                    ..Default::default()
                }
                .link_target(target)
                .clone()),
                MaybeReference::Actual(ApObject::Note(object)) => Ok(NewActivity {
                    kind: update.kind.into(),
                    uuid: uuid.clone(),
                    actor: update.actor.to_string(),
                    target_ap_id: object.id.map(|x| x.to_string()),
                    revoked: false,
                    ap_id: update
                        .id
                        .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                    ..Default::default()
                }
                .link_target(target)
                .clone()),
                MaybeReference::Actual(ApObject::Question(object)) => Ok(NewActivity {
                    kind: update.kind.into(),
                    uuid: uuid.clone(),
                    actor: update.actor.to_string(),
                    target_ap_id: Some(object.id),
                    revoked: false,
                    ap_id: update
                        .id
                        .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                    ..Default::default()
                }
                .link_target(target)
                .clone()),
                MaybeReference::Actual(ApObject::Collection(_)) => Ok(NewActivity {
                    kind: update.kind.into(),
                    uuid: uuid.clone(),
                    actor: update.actor.to_string(),
                    revoked: false,
                    ap_id: update
                        .id
                        .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                    ..Default::default()
                }),
                _ => Err(anyhow!(
                    "Update object not implemented in TryFrom<ApActivityTarget> for NewActivity"
                )),
            },
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
                _ => Err(anyhow!(
                    "Undo object not implemented in TryFrom<ApActivityTarget> for NewActivity"
                )),
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
                ap_to: like.to.into(),
                ..Default::default()
            }
            .link_target(target)
            .clone()),
            ApActivity::Remove(remove) => Ok(NewActivity {
                kind: remove.kind.into(),
                uuid: uuid.clone(),
                actor: remove.actor.to_string(),
                ap_to: remove.to.into(),
                cc: remove.cc.into(),
                target_ap_id: remove.object.reference(),
                revoked: false,
                ap_id: remove
                    .id
                    .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                ..Default::default()
            }
            .link_target(target)
            .clone()),
            ApActivity::Move(move_activity) => Ok(NewActivity {
                kind: move_activity.kind.into(),
                uuid: uuid.clone(),
                actor: move_activity.actor.to_string(),
                ap_to: move_activity.to.into(),
                cc: move_activity.cc.into(),
                target_ap_id: move_activity.object.reference(),
                revoked: false,
                ap_id: move_activity
                    .id
                    .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                ..Default::default()
            }
            .link_target(target)
            .clone()),

            _ => Err(anyhow!(
                "Unimplemented Activity type in TryFrom<ApActivityTarget> for NewActivity"
            )),
        }
    }
}

pub type ActorActivity = (Option<Actor>, Option<Actor>, ActivityType, ApAddress);

impl From<ActorActivity> for NewActivity {
    fn from((profile, remote_actor, kind, actor): ActorActivity) -> Self {
        let (ap_to, target_ap_id) = {
            if let Some(profile) = profile.clone() {
                (
                    Some(json!(vec![profile.as_id.clone()])),
                    Some(profile.as_id),
                )
            } else if let Some(remote_actor) = remote_actor.clone() {
                (
                    Some(json!(vec![remote_actor.as_id.clone()])),
                    Some(remote_actor.as_id),
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

pub trait TryFromExtendedActivity: Sized {
    type Error;
    fn try_from_extended_activity(activity: ExtendedActivity) -> Result<Self, Self::Error>;
}

pub type ExtendedActivity = (Activity, Option<Activity>, Option<Object>, Option<Actor>);

impl TryFromExtendedActivity for ApActivity {
    type Error = anyhow::Error;

    fn try_from_extended_activity(
        (activity, target_activity, target_object, target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        match activity.kind {
            ActivityType::Create => ApCreate::try_from_extended_activity((
                activity,
                target_activity,
                target_object,
                target_actor,
            ))
            .map(ApActivity::Create),
            ActivityType::Announce => ApAnnounce::try_from_extended_activity((
                activity,
                target_activity,
                target_object,
                target_actor,
            ))
            .map(ApActivity::Announce),
            ActivityType::Like => ApLike::try_from_extended_activity((
                activity,
                target_activity,
                target_object,
                target_actor,
            ))
            .map(|activity| ApActivity::Like(Box::new(activity))),
            ActivityType::Delete => {
                let note = ApNote::try_from(target_object.unwrap())?;
                ApDelete::try_from(note).map(|mut delete| {
                    delete.id = activity.ap_id;
                    ApActivity::Delete(Box::new(delete))
                })
            }
            ActivityType::Follow => ApFollow::try_from_extended_activity((
                activity,
                target_activity,
                target_object,
                target_actor,
            ))
            .map(ApActivity::Follow),
            ActivityType::Undo => ApUndo::try_from_extended_activity((
                activity,
                target_activity,
                target_object,
                target_actor,
            ))
            .map(|undo| ApActivity::Undo(Box::new(undo))),
            ActivityType::Accept => ApAccept::try_from_extended_activity((
                activity,
                target_activity,
                target_object,
                target_actor,
            ))
            .map(|accept| ApActivity::Accept(Box::new(accept))),
            ActivityType::Update => ApUpdate::try_from_extended_activity((
                activity,
                target_activity,
                target_object,
                target_actor,
            ))
            .map(ApActivity::Update),
            _ => {
                log::error!(
                    "Failed to match implemented activity in TryFrom for ApActivity\nACTIVITY: {activity:#?}\nTARGET_ACTIVITY: {target_activity:#?}\nTARGET_OBJECT: {target_object:#?}\nTARGET_ACTOR {target_actor:#?}"
                );
                Err(anyhow!("Failed to match implemented activity"))
            }
        }
    }
}

impl TryFromExtendedActivity for ApUpdate {
    type Error = anyhow::Error;

    fn try_from_extended_activity(
        (activity, _target_activity, _target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        // I wrote this with updating a collection of instruments in mind; for Actor or Object
        // updates, we probably want to do more here
        Ok(ApUpdate {
            context: Some(ApContext::default()),
            kind: ApUpdateType::default(),
            actor: activity.actor.clone().into(),
            id: Some(activity.ap_id.ok_or(anyhow!("Update must have an ap_id"))?),
            object: MaybeReference::None,
            ..Default::default()
        })
    }
}

impl TryFromExtendedActivity for ApAccept {
    type Error = anyhow::Error;

    fn try_from_extended_activity(
        (activity, target_activity, _target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        let follow = ApActivity::try_from_extended_activity((
            target_activity.ok_or(anyhow!("TARGET_ACTIVITY CANNOT BE NONE"))?,
            None,
            None,
            None,
        ))?;

        if let ApActivity::Follow(follow) = follow {
            Ok(ApAccept {
                context: Some(ApContext::default()),
                kind: ApAcceptType::default(),
                actor: activity.actor.clone().into(),
                id: Some(activity.ap_id.ok_or(anyhow!("ACCEPT MUST HAVE AN AP_ID"))?),
                object: MaybeReference::Actual(ApActivity::Follow(follow)),
            })
        } else {
            log::error!(
                "FAILED TO MATCH IMPLEMENTED ACCEPT IN TryFrom FOR ApAccept\n{activity:#?}"
            );
            Err(anyhow!("FAILED TO MATCH IMPLEMENTED ACCEPT"))
        }
    }
}

impl TryFromExtendedActivity for ApAnnounce {
    type Error = anyhow::Error;

    fn try_from_extended_activity(
        (activity, _target_activity, target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if activity.kind.to_string().to_lowercase().as_str() == "announce" {
            let object: ApObject = if let Some(object) = target_object.clone() {
                object.try_into()?
            } else {
                return Err(anyhow!("Unable to convert Object to ApObject"));
            };
            // let object = target_object.ok_or(anyhow!("INVALID ACTIVITY TYPE"))?;
            // let object = MaybeReference::Actual(ApObject::Note(ApNote::try_from(object)?));

            Ok(ApAnnounce {
                context: Some(ApContext::default()),
                kind: ApAnnounceType::default(),
                actor: activity.clone().actor.into(),
                id: Some(format!(
                    "https://{}/activities/{}",
                    *crate::SERVER_NAME,
                    activity.uuid
                )),
                to: activity.clone().ap_to.into(),
                cc: activity.cc.into(),
                published: activity.created_at.into(),
                object: object.into(),
                ephemeral: Some(Ephemeral {
                    created_at: Some(activity.created_at),
                    updated_at: Some(activity.updated_at),
                    ..Default::default()
                }),
            })
        } else {
            log::error!("NOT AN ANNOUNCE ACTIVITY");
            Err(anyhow::Error::msg("NOT AN ANNOUNCE ACTIVITY"))
        }
    }
}

// I suspect that any uses of this have now been redirected to the CoalescedActivity above,
// even if functions are still calling this impl. It would be good to remove this and clean
// up the function chains.
impl TryFromExtendedActivity for ApCreate {
    type Error = anyhow::Error;
    fn try_from_extended_activity(
        (activity, _target_activity, target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        let object: ApObject = if let Some(object) = target_object.clone() {
            object.try_into()?
        } else {
            return Err(anyhow!("Unable to convert Object to ApObject"));
        };

        // let note = {
        //     if let Some(object) = target_object.clone() {
        //         ApObject::Note(ApNote::try_from(object)?)
        //     } else {
        //         return Err(anyhow!("ACTIVITY MUST INCLUDE A NOTE OR REMOTE_NOTE"));
        //     }
        // };

        let instrument: MaybeMultiple<ApInstrument> = activity.instrument.into();
        let instrument = match instrument {
            MaybeMultiple::Single(instrument) => {
                if instrument.is_mls_welcome() {
                    vec![instrument].into()
                } else {
                    MaybeMultiple::None
                }
            }
            MaybeMultiple::Multiple(instruments) => instruments
                .into_iter()
                .filter(|x| x.is_mls_welcome())
                .collect::<Vec<ApInstrument>>()
                .into(),
            _ => MaybeMultiple::None,
        };

        Ok(ApCreate {
            context: Some(ApContext::default()),
            kind: ApCreateType::default(),
            actor: ApAddress::Address(activity.actor.clone()),
            id: activity.ap_id,
            object: object.into(),
            to: activity.ap_to.clone().into(),
            cc: activity.cc.into(),
            signature: None,
            published: Some(activity.created_at.into()),
            ephemeral: Some(Ephemeral {
                created_at: Some(activity.created_at),
                updated_at: Some(activity.updated_at),
                ..Default::default()
            }),
            instrument,
        })
    }
}

impl TryFromExtendedActivity for ApFollow {
    type Error = anyhow::Error;

    fn try_from_extended_activity(
        (activity, _target_activity, _target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if activity.kind.is_follow() {
            let target = activity
                .target_ap_id
                .ok_or(anyhow!("no target_ap_id on follow"))?;
            Ok(ApFollow {
                context: Some(ApContext::activity_streams()),
                kind: ApFollowType::default(),
                actor: activity.actor.into(),
                id: Some(activity.ap_id.ok_or(anyhow!("no follow as_id found"))?),
                to: activity.ap_to.into(),
                cc: activity.cc.into(),
                object: target.into(),
            })
        } else {
            log::error!("Not a Follow Activity");
            Err(anyhow!("Not a Follow Activity"))
        }
    }
}

impl TryFromExtendedActivity for ApLike {
    type Error = anyhow::Error;

    fn try_from_extended_activity(
        (activity, _target_activity, target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if !activity.kind.is_like() {
            return Err(anyhow!("NOT A LIKE ACTIVITY"));
        }

        let object = target_object.ok_or(anyhow!("no target object"))?;
        let note = ApNote::try_from(object)?;

        let (id, object): (String, MaybeReference<ApObject>) = (
            note.attributed_to.clone().to_string(),
            MaybeReference::Reference(note.id.ok_or(anyhow!("no note id"))?),
        );

        Ok(ApLike {
            context: Some(ApContext::activity_streams()),
            kind: ApLikeType::default(),
            actor: activity.actor.into(),
            id: activity.ap_id,
            to: MaybeMultiple::Single(ApAddress::Address(id)),
            object,
        })
    }
}

impl TryFromExtendedActivity for ApUndo {
    type Error = anyhow::Error;

    fn try_from_extended_activity(
        (activity, target_activity, target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        let target_activity = target_activity.ok_or(anyhow!("RECURSIVE CANNOT BE NONE"))?;
        let target_activity = ApActivity::try_from_extended_activity((
            target_activity.clone(),
            None,
            target_object,
            None,
        ))?;

        if !activity.kind.is_undo() {
            return Err(anyhow!("activity is not an undo"));
        }

        match target_activity {
            ApActivity::Follow(follow) => Ok(ApUndo {
                context: Some(ApContext::default()),
                kind: ApUndoType::default(),
                actor: activity.actor.clone().into(),
                id: activity.ap_id,
                object: MaybeReference::Actual(ApActivity::Follow(follow)),
            }),
            ApActivity::Like(like) => Ok(ApUndo {
                context: Some(ApContext::default()),
                kind: ApUndoType::default(),
                actor: activity.actor.clone().into(),
                id: activity.ap_id,
                object: MaybeReference::Actual(ApActivity::Like(like)),
            }),
            ApActivity::Announce(announce) => Ok(ApUndo {
                context: Some(ApContext::default()),
                kind: ApUndoType::default(),
                actor: activity.actor.clone().into(),
                id: activity.ap_id,
                object: MaybeReference::Actual(ApActivity::Announce(announce)),
            }),
            _ => {
                log::error!("FAILED TO MATCH IMPLEMENTED UNDO: {activity:#?}");
                Err(anyhow!("FAILED TO MATCH IMPLEMENTED UNDO"))
            }
        }
    }
}

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
        instrument: coalesced.recursive_instrument,
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
            instrument: coalesced.instrument.clone(),
        };

        let target_activity = target_to_main(coalesced.clone());
        let target_object: Option<Object> = coalesced.clone().try_into().ok();
        let target_actor: Option<Actor> = coalesced.try_into().ok();

        (activity, target_activity, target_object, target_actor)
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
    excluded_words: Vec<String>,
    direct: bool,
}

fn query_initial_block() -> String {
    indoc! {"
WITH main AS (
    SELECT DISTINCT ON (a.created_at)
        a.*,
        a2.created_at AS recursive_created_at,
        a2.updated_at AS recursive_updated_at,
        a2.kind AS recursive_kind,
        a2.uuid AS recursive_uuid,
        a2.actor AS recursive_actor,
        a2.ap_to AS recursive_ap_to,
        a2.cc AS recursive_cc,
        a2.target_activity_id AS recursive_target_activity_id,
        a2.target_ap_id AS recursive_target_ap_id,
        a2.revoked AS recursive_revoked,
        a2.ap_id AS recursive_ap_id,
        a2.reply AS recursive_reply,
        a2.target_object_id AS recursive_target_object_id,
        a2.actor_id AS recursive_actor_id,
        a2.target_actor_id AS recursive_target_actor_id,
        a2.instrument AS recursive_instrument,
        COALESCE(o.created_at, o2.created_at) AS object_created_at,
        COALESCE(o.updated_at, o2.updated_at) AS object_updated_at,
        COALESCE(o.ek_uuid, o2.ek_uuid) AS object_uuid,
        COALESCE(o.as_type, o2.as_type) AS object_type,
        COALESCE(o.as_published, o2.as_published) AS object_published,
        COALESCE(o.as_id, o2.as_id) AS object_as_id,
        COALESCE(o.as_url, o2.as_url) AS object_url,
        COALESCE(o.as_to, o2.as_to) AS object_to,
        COALESCE(o.as_cc, o2.as_cc) AS object_cc,
        COALESCE(o.as_tag, o2.as_tag) AS object_tag,
        COALESCE(o.as_attributed_to, o2.as_attributed_to) AS object_attributed_to,
        COALESCE(o.as_in_reply_to, o2.as_in_reply_to) AS object_in_reply_to,
        COALESCE(o.as_content, o2.as_content) AS object_content,
        COALESCE(o.ap_conversation, o2.ap_conversation) AS object_conversation,
        COALESCE(o.as_attachment, o2.as_attachment) AS object_attachment,
        COALESCE(o.as_summary, o2.as_summary) AS object_summary,
        COALESCE(o.as_end_time, o2.as_end_time) AS object_end_time,
        COALESCE(o.as_one_of, o2.as_one_of) AS object_one_of,
        COALESCE(o.as_any_of, o2.as_any_of) AS object_any_of,
        COALESCE(o.ap_voters_count, o2.ap_voters_count) AS object_voters_count,
        COALESCE(o.ap_sensitive, o2.ap_sensitive) AS object_sensitive,
        COALESCE(o.ek_metadata, o2.ek_metadata) AS object_metadata,
        COALESCE(o.ek_profile_id, o2.ek_profile_id) AS object_profile_id,
        COALESCE(o.ek_instrument, o2.ek_instrument) AS object_instrument,
        COALESCE(ta.created_at, ta2.created_at) AS actor_created_at,
        COALESCE(ta.updated_at, ta2.updated_at) AS actor_updated_at,
        COALESCE(ta.ek_uuid, ta2.ek_uuid) AS actor_uuid,
        COALESCE(ta.ek_username, ta2.ek_username) AS actor_username,
        COALESCE(ta.ek_summary_markdown, ta2.ek_summary_markdown) AS actor_summary_markdown,
        COALESCE(ta.ek_avatar_filename, ta2.ek_avatar_filename) AS actor_avatar_filename,
        COALESCE(ta.ek_banner_filename, ta2.ek_banner_filename) AS actor_banner_filename,
        COALESCE(ta.ek_private_key, ta2.ek_private_key) AS actor_private_key,
        COALESCE(ta.ek_password, ta2.ek_password) AS actor_password,
        COALESCE(ta.ek_client_public_key, ta2.ek_client_public_key) AS actor_client_public_key,
        COALESCE(ta.ek_client_private_key, ta2.ek_client_private_key) AS actor_client_private_key,
        COALESCE(ta.ek_salt, ta2.ek_salt) AS actor_salt,
        COALESCE(ta.ek_olm_pickled_account, ta2.ek_olm_pickled_account) AS actor_olm_pickled_account,
        COALESCE(ta.ek_olm_pickled_account_hash, ta2.ek_olm_pickled_account_hash) AS actor_olm_pickled_account_hash,
        COALESCE(ta.ek_olm_identity_key, ta2.ek_olm_identity_key) AS actor_olm_identity_key,
        COALESCE(ta.ek_webfinger, ta2.ek_webfinger) AS actor_webfinger,
        COALESCE(ta.ek_checked_at, ta2.ek_checked_at) AS actor_checked_at,
        COALESCE(ta.ek_hashtags, ta2.ek_hashtags) AS actor_hashtags,
        COALESCE(ta.as_type, ta2.as_type) AS actor_type,
        COALESCE(ta.as_context, ta2.as_context) AS actor_context,
        COALESCE(ta.as_id, ta2.as_id) AS actor_as_id,
        COALESCE(ta.as_name, ta2.as_name) AS actor_name,
        COALESCE(ta.as_preferred_username, ta2.as_preferred_username) AS actor_preferred_username,
        COALESCE(ta.as_summary, ta2.as_summary) AS actor_summary,
        COALESCE(ta.as_inbox, ta2.as_inbox) AS actor_inbox,
        COALESCE(ta.as_outbox, ta2.as_outbox) AS actor_outbox,
        COALESCE(ta.as_followers, ta2.as_followers) AS actor_followers,
        COALESCE(ta.as_following, ta2.as_following) AS actor_following,
        COALESCE(ta.as_liked, ta2.as_liked) AS actor_liked,
        COALESCE(ta.as_public_key, ta2.as_public_key) AS actor_public_key,
        COALESCE(ta.as_featured, ta2.as_featured) AS actor_featured,
        COALESCE(ta.as_featured_tags, ta2.as_featured_tags) AS actor_featured_tags,
        COALESCE(ta.as_url, ta2.as_url) AS actor_url,
        COALESCE(ta.as_published, ta2.as_published) AS actor_published,
        COALESCE(ta.as_tag, ta2.as_tag) AS actor_tag,
        COALESCE(ta.as_attachment, ta2.as_attachment) AS actor_attachment,
        COALESCE(ta.as_endpoints, ta2.as_endpoints) AS actor_endpoints,
        COALESCE(ta.as_icon, ta2.as_icon) AS actor_icon,
        COALESCE(ta.as_image, ta2.as_image) AS actor_image,
        COALESCE(ta.as_also_known_as, ta2.as_also_known_as) AS actor_also_known_as,
        COALESCE(ta.as_discoverable, ta2.as_discoverable) AS actor_discoverable,
        COALESCE(ta.ap_capabilities, ta2.ap_capabilities) AS actor_capabilities,
        COALESCE(ta.ek_keys, ta2.ek_keys) AS actor_keys,
        COALESCE(ta.ek_last_decrypted_activity, ta2.ek_last_decrypted_activity) AS actor_last_decrypted_activity,
        COALESCE(ta.ap_manually_approves_followers, ta2.ap_manually_approves_followers) AS actor_manually_approves_followers,
        COALESCE(ta.ek_mls_credentials, ta2.ek_mls_credentials) AS actor_mls_credentials,
        COALESCE(ta.ek_mls_storage, ta2.ek_mls_storage) AS actor_mls_storage,
        COALESCE(ta.ek_mls_storage_hash, ta2.ek_mls_storage_hash) AS actor_mls_storage_hash,
        COALESCE(ta.ek_muted_terms, ta2.ek_muted_terms) AS actor_muted_terms
"}.to_string()
}

fn query_end_block(mut query: String) -> String {
    query.push_str(indoc! {"
SELECT DISTINCT
    m.*,
    COALESCE(JSONB_AGG(jsonb_build_object('id', ac.as_id, 'name', ac.as_name, 'tag', ac.as_tag, 'url', ac.as_url, 'icon', ac.as_icon, 'preferredUsername', ac.as_preferred_username)) FILTER (WHERE a.actor IS NOT NULL
            AND a.kind = 'announce'), '[]') AS object_announcers,
    COALESCE(JSONB_AGG(jsonb_build_object('id', ac.as_id, 'name', ac.as_name, 'tag', ac.as_tag, 'url', ac.as_url, 'icon', ac.as_icon, 'preferredUsername', ac.as_preferred_username)) FILTER (WHERE a.actor IS NOT NULL
            AND a.kind = 'like'), '[]') AS object_likers,
    JSONB_AGG(DISTINCT jsonb_build_object('id', ac2.as_id, 'name', ac2.as_name, 'tag', ac2.as_tag, 'url', ac2.as_url, 'icon', ac2.as_icon, 'preferredUsername', ac2.as_preferred_username, 'webfinger', ac2.ek_webfinger)) AS object_attributed_to_profiles,
    announced.object_announced,
    liked.object_liked,
    vaulted.*,
    mls.*
FROM
    main m
    LEFT JOIN attributed_actors ac2 ON ac2.main_id = m.id
    LEFT JOIN activities a ON (a.target_ap_id = m.object_as_id
            AND NOT a.revoked
            AND (a.kind = 'announce'
                OR a.kind = 'like'))
    LEFT JOIN actors ac ON (ac.as_id = a.actor)
    LEFT JOIN announced ON m.id = announced.id
    LEFT JOIN liked ON m.id = liked.id
    LEFT JOIN vaulted ON m.id = vaulted.vault_activity_id
    LEFT JOIN mls ON mls.mls_group_id_conversation = m.object_conversation
GROUP BY
    m.id,
    m.created_at,
    m.updated_at,
    m.kind,
    m.uuid,
    m.actor,
    m.ap_to,
    m.cc,
    m.target_activity_id,
    m.target_ap_id,
    m.revoked,
    m.ap_id,
    m.reply,
    m.instrument,
    m.recursive_created_at,
    m.recursive_updated_at,
    m.recursive_kind,
    m.recursive_uuid,
    m.recursive_actor,
    m.recursive_ap_to,
    m.recursive_cc,
    m.recursive_target_activity_id,
    m.recursive_target_ap_id,
    m.recursive_revoked,
    m.recursive_ap_id,
    m.recursive_reply,
    m.recursive_target_object_id,
    m.recursive_actor_id,
    m.recursive_target_actor_id,
    m.recursive_instrument,
    m.object_created_at,
    m.object_updated_at,
    m.object_uuid,
    m.object_type,
    m.object_published,
    m.object_as_id,
    m.object_url,
    m.object_to,
    m.object_cc,
    m.object_tag,
    m.object_attributed_to,
    m.object_in_reply_to,
    m.object_content,
    m.object_conversation,
    m.object_attachment,
    m.object_summary,
    m.object_end_time,
    m.object_one_of,
    m.object_any_of,
    m.object_voters_count,
    m.object_sensitive,
    m.object_metadata,
    m.object_profile_id,
    m.object_instrument,
    m.raw,
    m.actor_id,
    m.target_actor_id,
    m.log,
    m.target_object_id,
    m.actor_created_at,
    m.actor_updated_at,
    m.actor_uuid,
    m.actor_username,
    m.actor_summary_markdown,
    m.actor_avatar_filename,
    m.actor_banner_filename,
    m.actor_private_key,
    m.actor_password,
    m.actor_client_public_key,
    m.actor_client_private_key,
    m.actor_salt,
    m.actor_olm_pickled_account,
    m.actor_olm_pickled_account_hash,
    m.actor_olm_identity_key,
    m.actor_webfinger,
    m.actor_checked_at,
    m.actor_hashtags,
    m.actor_type,
    m.actor_context,
    m.actor_as_id,
    m.actor_name,
    m.actor_preferred_username,
    m.actor_summary,
    m.actor_inbox,
    m.actor_outbox,
    m.actor_followers,
    m.actor_following,
    m.actor_liked,
    m.actor_public_key,
    m.actor_featured,
    m.actor_featured_tags,
    m.actor_url,
    m.actor_published,
    m.actor_tag,
    m.actor_attachment,
    m.actor_endpoints,
    m.actor_icon,
    m.actor_image,
    m.actor_also_known_as,
    m.actor_discoverable,
    m.actor_capabilities,
    m.actor_keys,
    m.actor_last_decrypted_activity,
    m.actor_manually_approves_followers,
    m.actor_mls_credentials,
    m.actor_mls_storage,
    m.actor_mls_storage_hash,
    m.actor_muted_terms,
    announced.object_announced,
    liked.object_liked,
    vaulted.vault_id,
    vaulted.vault_created_at,
    vaulted.vault_updated_at,
    vaulted.vault_uuid,
    vaulted.vault_owner_as_id,
    vaulted.vault_activity_id,
    vaulted.vault_data,
    mls.mls_group_id_id,
    mls.mls_group_id_created_at,
    mls.mls_group_id_updated_at,
    mls.mls_group_id_uuid,
    mls.mls_group_id_actor_id,
    mls.mls_group_id_conversation,
    mls.mls_group_id_mls_group
"});
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

    query.push_str(indoc! {"
FROM activities a 
    LEFT JOIN objects o ON (o.id = a.target_object_id)
    LEFT JOIN actors ta ON (ta.id = a.target_actor_id)
    LEFT JOIN activities a2 ON (a.target_activity_id = a2.id)
    LEFT JOIN objects o2 ON (a2.target_object_id = o2.id)
    LEFT JOIN actors ta2 ON (ta2.id = a2.target_actor_id)
    LEFT JOIN actors ac ON (a.actor_id = ac.id)
"});

    let mut combined_excluded_words = vec![];
    if let Some(actor_profile) = profile {
        if let Value::Array(muted_terms_array) = actor_profile.ek_muted_terms.clone() {
            for term_value in muted_terms_array {
                if let Value::String(term_str) = term_value {
                    combined_excluded_words.push(term_str.clone());
                }
            }
        }
    }

    query.push_str(&format!(
        "WHERE COALESCE(o.as_content, o2.as_content, '') !~* ('\\m#?(' || {} || ')\\M') ",
        param_gen()
    ));

    if params.activity_as_id.is_some() {
        query.push_str(&format!("AND a.ap_id = {}), ", param_gen()));
    } else if params.activity_uuid.is_some() {
        query.push_str(&format!("AND a.uuid = {}), ", param_gen()));
    } else if params.activity_id.is_some() {
        query.push_str(&format!("AND a.id = {}), ", param_gen()));
    } else {
        if filters.clone().and_then(|x| x.view).is_some() {
            query.push_str(
                "AND a.kind IN ('announce','create') \
                 AND NOT o.as_type IN ('tombstone') ",
            );
        } else {
            query.push_str(
                "AND a.kind IN ('announce','create','undo','like','follow','accept','delete') ",
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
                params.date = DateTime::from_timestamp_micros(min).unwrap();
                query.push_str(&format!("AND a.created_at > {} ", param_gen()));
            }
        } else if let Some(max) = max {
            params.date = DateTime::from_timestamp_micros(max).unwrap();
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
                        }
                    }
                }
            } else {
                params.to.extend((*PUBLIC_COLLECTION).clone());
                if let Some(profile) = profile.clone() {
                    params.to.extend(vec![profile.as_id.clone()]);
                    params.from.extend(vec![profile.as_id]);
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

            combined_excluded_words.extend(params.excluded_words);
        }

        if (min.is_some() && min.unwrap() == 0) || params.conversation.clone().is_some() {
            query.push_str(&format!("ORDER BY created_at ASC LIMIT {}), ", param_gen()));
        } else {
            query.push_str(&format!(
                "ORDER BY created_at DESC LIMIT {}), ",
                param_gen()
            ));
        }
    };

    combined_excluded_words.sort_unstable();
    combined_excluded_words.dedup();

    params.excluded_words = combined_excluded_words;

    if profile.is_some() {
        query.push_str(&format!(
            indoc! {"
announced AS (
    SELECT
        m.id,
        a.ap_id AS object_announced
    FROM
        main m
        LEFT JOIN activities a ON (a.target_ap_id = m.object_as_id 
            AND NOT a.revoked 
            AND a.kind = 'announce'
            AND a.actor_id = {})
    GROUP BY
        m.id,
        a.ap_id
),
liked AS (
    SELECT
        m.id,
        a.ap_id AS object_liked
    FROM
        main m
        LEFT JOIN activities a ON (a.target_ap_id = m.object_as_id
            AND NOT a.revoked
            AND a.kind = 'like'
            AND  a.actor_id = {})
    GROUP BY
        m.id,
        a.ap_id
),
vaulted AS (
    SELECT
        v.id AS vault_id,
        v.created_at AS vault_created_at,
        v.updated_at AS vault_updated_at,
        v.uuid AS vault_uuid,
        v.owner_as_id AS vault_owner_as_id,
        v.activity_id AS vault_activity_id,
        v.data AS vault_data
    FROM
        main m
        LEFT JOIN vault v ON (v.activity_id = m.id
            AND (m.actor = {} OR m.ap_to @> {})
            AND v.owner_as_id = {})
),
mls AS (
    SELECT
        mgc.id AS mls_group_id_id,
        mgc.created_at AS mls_group_id_created_at,
        mgc.updated_at AS mls_group_id_updated_at,
        mgc.uuid AS mls_group_id_uuid,
        mgc.actor_id AS mls_group_id_actor_id,
        mgc.conversation AS mls_group_id_conversation,
        mgc.mls_group AS mls_group_id_mls_group
    FROM
        main m
    LEFT JOIN mls_group_conversations mgc ON (m.object_conversation = mgc.conversation
            AND mgc.actor_id = {})
),
"},
            param_gen(),
            param_gen(),
            param_gen(),
            param_gen(),
            param_gen(),
            param_gen(),
        ));
    } else {
        query.push_str(indoc! {"
announced AS (
    SELECT
        m.id,
        NULL AS object_announced 
    FROM
        main m
), 
liked AS (
    SELECT
        m.id,
        NULL AS object_liked 
    FROM
        main m
), 
vaulted AS (
    SELECT
        NULL AS vault_id,
        NULL AS vault_created_at,
        NULL AS vault_updated_at, 
        NULL AS vault_uuid,
        NULL AS vault_owner_as_id,
        NULL::INT AS vault_activity_id, 
        NULL AS vault_data 
    FROM
        main m
), 
mls AS (
    SELECT
        NULL AS mls_group_id_id,
        NULL AS mls_group_id_created_at,
        NULL AS mls_group_id_updated_at,
        NULL AS mls_group_id_uuid,
        NULL AS mls_group_id_actor_id,
        NULL AS mls_group_id_conversation,
        NULL AS mls_group_id_mls_group
    FROM
        main m
),
"});
    }

    query.push_str(indoc! {"
attributed_actors AS (
    SELECT DISTINCT
        m.id AS main_id,
        ac2.*
    FROM
        main m
    CROSS JOIN LATERAL (
        SELECT
            unnested.attr_to
        FROM (
            SELECT
                CASE jsonb_typeof(m.object_attributed_to)
                WHEN 'string' THEN
                    m.object_attributed_to #>> '{}'
                ELSE
                    NULL
                END AS attr_to
        UNION ALL
        SELECT
            jsonb_array_elements_text(m.object_attributed_to) AS attr_to
        WHERE
            jsonb_typeof(m.object_attributed_to) = 'array') AS unnested
    WHERE
        unnested.attr_to IS NOT NULL) AS attributed
    JOIN actors ac2 ON ac2.as_id = attributed.attr_to
) 
"});

    let mut query = query_end_block(query);

    if params.conversation.is_some() {
        query.push_str("ORDER BY m.created_at ASC");
    } else {
        query.push_str("ORDER BY m.created_at DESC");
    }

    params.query = Some(query);

    params
}

//pub async fn add_log_by_as_id(conn: Option<&Db>, as_id: String, entry: Value) -> Result<usize> {
pub async fn add_log_by_as_id<C: DbRunner>(conn: &C, as_id: String, entry: Value) -> Result<usize> {
    use diesel::sql_types::{Jsonb, Text};

    conn.run(move |c| {
        let mut query = sql_query(
            "UPDATE activities a SET log = COALESCE(a.log, '[]'::jsonb) || $1::jsonb WHERE ap_id = $2",
        )
        .into_boxed::<DbType>();
        query = query.bind::<Jsonb, _>(entry);
        query = query.bind::<Text, _>(as_id);
        query.execute(c)
    })
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn get_activities_coalesced<C: DbRunner>(
    conn: &C,
    limit: i32,
    min: Option<i64>,
    max: Option<i64>,
    profile: Option<Actor>,
    filters: Option<TimelineFilters>,
    as_id: Option<String>,
    uuid: Option<String>,
    id: Option<i32>,
) -> Result<Vec<CoalescedActivity>> {
    let params = build_main_query(&filters, limit, min, max, &profile, as_id, uuid, id);

    log::debug!("QUERY\n{}", params.query.clone().unwrap_or("".to_string()));

    let query = sql_query(params.query.clone().unwrap()).into_boxed::<DbType>();

    conn.run(move |c| {
        let query = bind_params(query, params, &profile);
        query.load::<CoalescedActivity>(c)
    })
    .await
}

fn bind_params<'a>(
    query: BoxedSqlQuery<'a, DbType, SqlQuery>,
    params: QueryParams,
    profile: &Option<Actor>,
) -> BoxedSqlQuery<'a, DbType, SqlQuery> {
    use diesel::sql_types::{Array, Integer, Jsonb, Text, Timestamptz};
    let mut query = query;

    let terms = params.excluded_words.join("|");
    log::debug!("SETTING EXCLUSIONS: {terms}");
    query = query.bind::<Text, _>(terms);

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
            //log::debug!("SETTING DIRECT");
            //query = query.bind::<Array<Text>, _>((*PUBLIC_COLLECTION).clone());
            //query = query.bind::<Array<Text>, _>((*PUBLIC_COLLECTION).clone());
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

pub async fn create_activity<C: DbRunner>(conn: &C, mut activity: NewActivity) -> Result<Activity> {
    activity = activity.link_actor(conn).await;

    let operation = move |c: &mut PgConnection| {
        diesel::insert_into(activities::table)
            .values(&activity)
            .on_conflict(activities::ap_id)
            .do_update()
            .set(&activity)
            .get_result::<Activity>(c)
    };

    conn.run(operation).await
}

pub async fn get_announcers<C: DbRunner>(
    conn: &C,
    min: Option<i64>,
    max: Option<i64>,
    limit: Option<u8>,
    target_ap_id: String,
) -> Result<Vec<Actor>> {
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
        query.get_results(c)
    })
    .await
}

pub async fn get_likers<C: DbRunner>(
    conn: &C,
    min: Option<i64>,
    max: Option<i64>,
    limit: Option<u8>,
    target_ap_id: String,
) -> Result<Vec<Actor>> {
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
        query.get_results(c)
    })
    .await
}

pub async fn revoke_activities_by_object_as_id<C: DbRunner>(
    conn: &C,
    as_id: String,
) -> Result<Vec<Activity>> {
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
        .get_results::<Activity>(c)
    })
    .await
}

pub async fn revoke_activity_by_uuid<C: DbRunner>(conn: &C, uuid: String) -> Result<Activity> {
    //pub async fn revoke_activity_by_uuid(conn: Option<&Db>, uuid: String) -> Result<Activity> {
    let operation = move |c: &mut PgConnection| {
        diesel::update(activities::table.filter(activities::uuid.eq(uuid)))
            .set(activities::revoked.eq(true))
            .get_result::<Activity>(c)
    };

    //crate::db::run_db_op(conn, &crate::POOL, operation).await
    conn.run(operation).await
}

pub async fn revoke_activity_by_apid<C: DbRunner>(conn: &C, ap_id: String) -> Result<Activity> {
    //pub async fn revoke_activity_by_apid(conn: Option<&Db>, ap_id: String) -> Result<Activity> {
    let operation = move |c: &mut diesel::PgConnection| {
        diesel::update(activities::table.filter(activities::ap_id.eq(ap_id)))
            .set(activities::revoked.eq(true))
            .get_result::<Activity>(c)
    };

    conn.run(operation).await
    //crate::db::run_db_op(conn, &crate::POOL, operation).await
}

pub async fn set_activity_log_by_apid<C: DbRunner>(
    conn: &C,
    ap_id: String,
    log: Value,
) -> Result<Activity> {
    conn.run(move |c| {
        diesel::update(activities::table.filter(activities::ap_id.eq(ap_id)))
            .set(activities::log.eq(log))
            .get_result::<Activity>(c)
    })
    .await
}

pub async fn get_activity_by_ap_id<C: DbRunner>(
    conn: &C,
    ap_id: String,
) -> Result<Option<ExtendedActivity>> {
    let activities =
        get_activities_coalesced(conn, 1, None, None, None, None, Some(ap_id), None, None).await?;
    Ok(activities.first().cloned().map(ExtendedActivity::from))
}

pub async fn get_unrevoked_activity_by_kind_actor_id_and_target_ap_id<C: DbRunner>(
    conn: &C,
    kind: ActivityType,
    actor_id: i32,
    target_ap_id: String,
) -> Result<Option<Activity>> {
    conn.run(move |c| {
        activities::table
            .filter(activities::revoked.eq(false))
            .filter(activities::kind.eq(kind))
            .filter(activities::actor_id.eq(actor_id))
            .filter(activities::target_ap_id.eq(target_ap_id))
            .filter(activities::revoked.eq(false))
            .first::<Activity>(c)
            .optional()
    })
    .await
}

pub async fn get_activity_by_kind_actor_id_and_target_ap_id<C: DbRunner>(
    conn: &C,
    kind: ActivityType,
    actor_id: i32,
    target_ap_id: String,
) -> Result<Option<Activity>> {
    conn.run(move |c| {
        activities::table
            .filter(activities::revoked.eq(false))
            .filter(activities::kind.eq(kind))
            .filter(activities::actor_id.eq(actor_id))
            .filter(activities::target_ap_id.eq(target_ap_id))
            .first::<Activity>(c)
            .optional()
    })
    .await
}

pub async fn lookup_activity_id_by_as_id<C: DbRunner>(conn: &C, as_id: String) -> Result<i32> {
    conn.run(move |c| {
        activities::table
            .filter(activities::ap_id.eq(as_id))
            .select(activities::id)
            .first::<i32>(c)
    })
    .await
}

pub async fn get_activity<C: DbRunner>(conn: &C, id: i32) -> Result<Option<ExtendedActivity>> {
    let activities =
        get_activities_coalesced(conn, 1, None, None, None, None, None, None, Some(id)).await?;
    Ok(activities.first().cloned().map(ExtendedActivity::from))
}

pub async fn get_outbox_count_by_actor_id<C: DbRunner>(conn: &C, actor_id: i32) -> Result<i64> {
    conn.run(move |c| {
        activities::table
            .filter(activities::revoked.eq(false))
            .filter(activities::actor_id.eq(actor_id))
            .filter(
                activities::kind
                    .eq(ActivityType::Create)
                    .or(activities::kind.eq(ActivityType::Announce)),
            )
            .count()
            .get_result::<i64>(c)
    })
    .await
}

pub async fn update_target_object<C: DbRunner>(
    conn: &C,
    activity: Activity,
    object: Object,
) -> Result<usize> {
    let operation = move |c: &mut PgConnection| {
        diesel::update(activities::table.find(activity.id))
            .set(activities::target_object_id.eq(object.id))
            .execute(c)
    };

    conn.run(operation).await
}

pub async fn delete_activities_by_domain_pattern<C: DbRunner>(
    conn: &C,
    domain_pattern: String,
) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("DELETE FROM activities WHERE actor COLLATE \"C\" LIKE $1")
            .bind::<Text, _>(format!("https://{domain_pattern}/%"))
            .execute(c)
    };

    conn.run(operation).await
}

pub async fn delete_activities_by_actor<C: DbRunner>(conn: &C, actor: String) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("DELETE FROM activities WHERE actor = $1")
            .bind::<Text, _>(actor)
            .execute(c)
    };

    conn.run(operation).await
}

// pub async fn update_target_object(
//     conn: Option<&Db>,
//     activity: Activity,
//     object: Object,
// ) -> Result<usize> {
//     let operation = move |c: &mut PgConnection| {
//         diesel::update(activities::table.find(activity.id))
//             .set(activities::target_object_id.eq(object.id))
//             .execute(c)
//     };

//     crate::db::run_db_op(conn, &crate::POOL, operation).await
// }

// pub async fn delete_activities_by_domain_pattern(
//     conn: Option<&Db>,
//     domain_pattern: String,
// ) -> Result<usize> {
//     let operation = move |c: &mut diesel::PgConnection| {
//         use diesel::sql_types::Text;

//         sql_query("DELETE FROM activities WHERE actor COLLATE \"C\" LIKE $1")
//             .bind::<Text, _>(format!("https://{domain_pattern}/%"))
//             .execute(c)
//     };

//     crate::db::run_db_op(conn, &crate::POOL, operation).await
// }

// pub async fn delete_activities_by_actor(conn: Option<&Db>, actor: String) -> Result<usize> {
//     let operation = move |c: &mut diesel::PgConnection| {
//         use diesel::sql_types::Text;

//         sql_query("DELETE FROM activities WHERE actor = $1")
//             .bind::<Text, _>(actor)
//             .execute(c)
//     };

//     crate::db::run_db_op(conn, &crate::POOL, operation).await
// }
