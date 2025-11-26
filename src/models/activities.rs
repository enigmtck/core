use crate::db::runner::DbRunner;
use crate::db::DbType;
use crate::helper::get_activity_ap_id_from_uuid;
use crate::models::actors::{get_actor_by_as_id, Actor};
use crate::models::coalesced_activity::CoalescedActivity;
use crate::models::objects::{Object, ObjectType};
use crate::schema::{activities, actors};
use crate::server::InboxView;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use convert_case::{Case, Casing};
use diesel::sql_types::{Array, Bool, Integer, Nullable, Text};
use diesel::{prelude::*, sql_query};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use jdt_activity_pub::ApBlockType;
use jdt_activity_pub::ApMoveType;
use jdt_activity_pub::ApRejectType;
use jdt_activity_pub::ApRemoveType;
use jdt_activity_pub::MaybeMultiple;
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{
    ApAccept, ApAcceptType, ApActivity, ApActor, ApAddress, ApAnnounce, ApAnnounceType, ApContext,
    ApCreate, ApCreateType, ApDateTime, ApDelete, ApDeleteType, ApFollow, ApFollowType,
    ApInstrument, ApLike, ApLikeType, ApNote, ApObject, ApQuestion, ApReject, ApUndo, ApUndoType,
    ApUpdateType, Ephemeral,
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
    Reject,
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

    pub fn is_reject(&self) -> bool {
        self == &ActivityType::Reject
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
            "reject" => Ok(ActivityType::Reject),
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

impl From<ApRejectType> for ActivityType {
    fn from(_: ApRejectType) -> Self {
        ActivityType::Reject
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

#[derive(Eq, PartialEq, Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct TimelineFilters {
    pub view: Option<TimelineView>,
    pub hashtags: Vec<String>,
    pub username: Option<String>,
    pub conversation: Option<String>,
    pub excluded_words: Vec<String>,
    pub direct: bool,
    /// Filter by object type (e.g., Article, Note, Question)
    /// When None, all types are returned
    pub object_type: Option<ObjectType>,
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
    pub as_published: Option<DateTime<Utc>>,
}

impl Activity {
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
    pub as_published: Option<DateTime<Utc>>,
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
            as_published: None,
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
            ApActivity::Reject(reject) => {
                if let Some(ActivityTarget::Activity(follow)) = target.clone() {
                    Ok(NewActivity {
                        kind: reject.kind.into(),
                        uuid: uuid.clone(),
                        actor: reject.actor.to_string(),
                        target_ap_id: follow.ap_id,
                        revoked: false,
                        ap_id: reject
                            .id
                            .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                        ..Default::default()
                    }
                    .link_target(target)
                    .clone())
                } else {
                    Err(anyhow!(
                        "Unable to locate Follow in TryFrom<ApActivityTarget> for NewActivity (Reject)"
                    ))
                }
            }
            ApActivity::Update(update) => match update.object.clone() {
                MaybeReference::Actual(ApObject::Actor(actor)) => Ok(NewActivity {
                    kind: update.kind.clone().into(),
                    uuid: uuid.clone(),
                    actor: update.actor.to_string(),
                    ap_to: update.to.clone().into(),
                    target_ap_id: actor.id.map(|x| x.to_string()),
                    revoked: false,
                    ap_id: update
                        .id
                        .clone()
                        .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                    raw: Some(json!(update.clone())),
                    as_published: update.published.map(|p| *p),
                    ..Default::default()
                }
                .link_target(target)
                .clone()),
                MaybeReference::Actual(ApObject::Note(object)) => Ok(NewActivity {
                    kind: update.kind.into(),
                    uuid: uuid.clone(),
                    actor: update.actor.to_string(),
                    ap_to: update.to.clone().into(),
                    target_ap_id: object.id.map(|x| x.to_string()),
                    revoked: false,
                    ap_id: update
                        .id
                        .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                    as_published: update.published.map(|p| *p),
                    ..Default::default()
                }
                .link_target(target)
                .clone()),
                MaybeReference::Actual(ApObject::Question(object)) => Ok(NewActivity {
                    kind: update.kind.into(),
                    uuid: uuid.clone(),
                    actor: update.actor.to_string(),
                    ap_to: update.to.clone().into(),
                    target_ap_id: object.id.map(|x| x.to_string()),
                    revoked: false,
                    ap_id: update
                        .id
                        .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                    as_published: update.published.map(|p| *p),
                    ..Default::default()
                }
                .link_target(target)
                .clone()),
                MaybeReference::Actual(ApObject::Article(object)) => Ok(NewActivity {
                    kind: update.kind.into(),
                    uuid: uuid.clone(),
                    actor: update.actor.to_string(),
                    ap_to: update.to.clone().into(),
                    target_ap_id: object.id.map(|x| x.to_string()),
                    revoked: false,
                    ap_id: update
                        .id
                        .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                    as_published: update.published.map(|p| *p),
                    ..Default::default()
                }
                .link_target(target)
                .clone()),
                MaybeReference::Actual(ApObject::Collection(_)) => Ok(NewActivity {
                    kind: update.kind.into(),
                    uuid: uuid.clone(),
                    actor: update.actor.to_string(),
                    ap_to: update.to.clone().into(),
                    revoked: false,
                    ap_id: update
                        .id
                        .map_or(Some(get_activity_ap_id_from_uuid(uuid)), Some),
                    as_published: update.published.map(|p| *p),
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
                let target_obj = target_object.unwrap();
                // Try to convert to Note first, then Question
                let delete = if let Ok(note) = ApNote::try_from(target_obj.clone()) {
                    ApDelete::try_from(note)?
                } else if let Ok(question) = ApQuestion::try_from(target_obj.clone()) {
                    ApDelete::try_from(question)?
                } else {
                    return Err(anyhow!("Delete target must be Note or Question"));
                };

                Ok(ApActivity::Delete(Box::new(ApDelete {
                    id: activity.ap_id,
                    ..delete
                })))
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
            ActivityType::Reject => ApReject::try_from_extended_activity((
                activity,
                target_activity,
                target_object,
                target_actor,
            ))
            .map(|reject| ApActivity::Reject(Box::new(reject))),
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
        (activity, _target_activity, target_object, target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        // Reconstruct the object from target_object or target_actor
        let object = if let Some(obj) = target_object {
            MaybeReference::Actual(ApObject::try_from(obj)?)
        } else if let Some(actor) = target_actor {
            MaybeReference::Actual(ApObject::Actor(ApActor::from(actor)))
        } else {
            MaybeReference::None
        };

        // Extract to addresses from activity.ap_to
        let to: MaybeMultiple<ApAddress> = activity
            .ap_to
            .map(|v| serde_json::from_value(v).unwrap_or_default())
            .unwrap_or_default();

        // Convert as_published to ApDateTime
        let published = activity.as_published.map(ApDateTime::from);

        Ok(ApUpdate {
            context: Some(ApContext::default()),
            kind: ApUpdateType::default(),
            actor: activity.actor.clone().into(),
            id: Some(activity.ap_id.ok_or(anyhow!("Update must have an ap_id"))?),
            to,
            object,
            published,
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

impl TryFromExtendedActivity for ApReject {
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
            Ok(ApReject {
                context: Some(ApContext::default()),
                kind: ApRejectType::default(),
                actor: activity.actor.clone().into(),
                id: Some(activity.ap_id.ok_or(anyhow!("REJECT MUST HAVE AN AP_ID"))?),
                object: MaybeReference::Actual(ApActivity::Follow(follow)),
            })
        } else {
            log::error!(
                "FAILED TO MATCH IMPLEMENTED REJECT IN TryFrom FOR ApReject\n{activity:#?}"
            );
            Err(anyhow!("FAILED TO MATCH IMPLEMENTED REJECT"))
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
        as_published: None,
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
            //raw: coalesced.raw.clone(),
            raw: None,
            target_object_id: coalesced.target_object_id,
            actor_id: coalesced.actor_id,
            target_actor_id: coalesced.target_actor_id,
            //log: coalesced.log.clone(),
            log: None,
            instrument: coalesced.instrument.clone(),
            as_published: coalesced.as_published,
        };

        let target_activity = target_to_main(coalesced.clone());
        let target_object: Option<Object> = coalesced.clone().try_into().ok();
        let target_actor: Option<Actor> = coalesced.try_into().ok();

        (activity, target_activity, target_object, target_actor)
    }
}

#[derive(Debug)]
struct TimelineQueryParams {
    excluded_words: String,
    is_local_view: bool,
    to_addresses: Vec<String>,
    from_addresses: Vec<String>,
    hashtags: Vec<String>,
    max_date: String,
    min_date: String,
    order_asc: bool,
    limit: i32,
    outbox_username: String,
    profile_actor_id: String,
}

impl Default for TimelineQueryParams {
    fn default() -> Self {
        Self {
            excluded_words: String::new(),
            is_local_view: false,
            to_addresses: vec![],
            from_addresses: vec![],
            hashtags: vec![],
            max_date: "NULL".to_string(),
            min_date: "NULL".to_string(),
            order_asc: false,
            limit: 0,
            outbox_username: "NULL".to_string(),
            profile_actor_id: "NULL".to_string(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_timeline_query<'a>(
    filters: &'a Option<TimelineFilters>,
    limit: i32,
    min: Option<i64>,
    max: Option<i64>,
    profile: &'a Option<Actor>,
    _activity_as_id: Option<String>,
    _activity_uuid: Option<String>,
    _activity_id: Option<i32>,
) -> (&'static str, TimelineQueryParams) {
    let mut params = TimelineQueryParams {
        limit,
        ..Default::default()
    };

    let mut combined_excluded_words = vec![];
    if let Some(actor_profile) = profile {
        params.profile_actor_id = actor_profile.id.to_string();
        if let Value::Array(muted_terms_array) = actor_profile.ek_muted_terms.clone() {
            for term_value in muted_terms_array {
                if let Value::String(term_str) = term_value {
                    combined_excluded_words.push(term_str);
                }
            }
        }
    }

    if let Some(filters) = filters.clone() {
        if filters.conversation.is_some() || (min.is_some() && min.unwrap() == 0) {
            params.order_asc = true;
        } else if let Some(username) = filters.username {
            params.to_addresses.extend((*PUBLIC_COLLECTION).clone());
            params.outbox_username = username;
        } else {
            match filters.view.clone() {
                Some(TimelineView::Local) => {
                    params.is_local_view = true;
                }
                Some(TimelineView::Home(leaders)) if profile.is_some() => {
                    let profile = profile.clone().unwrap();
                    params.to_addresses.extend(leaders);
                    params.to_addresses.extend(vec![profile.as_id.clone()]);
                    params.from_addresses.extend(vec![profile.as_id.clone()]);
                }
                Some(TimelineView::Direct) if profile.is_some() => {
                    let profile = profile.clone().unwrap();
                    params.to_addresses.extend(vec![profile.as_id.clone()]);
                    params.from_addresses.extend(vec![profile.as_id.clone()]);
                }
                Some(TimelineView::Global)
                | Some(TimelineView::Direct)
                | Some(TimelineView::Home(_))
                | None => {
                    //Default to a Public view
                    params.to_addresses.extend((*PUBLIC_COLLECTION).clone());
                }
            };
        }

        params.hashtags.extend(filters.hashtags);
        combined_excluded_words.extend(filters.excluded_words);
    }

    if let Some(min_val) = min {
        if min_val != 0 {
            if let Some(dt) = DateTime::from_timestamp_micros(min_val) {
                params.min_date = dt.to_rfc3339();
            }
        }
    } else if let Some(max_val) = max {
        if let Some(dt) = DateTime::from_timestamp_micros(max_val) {
            params.max_date = dt.to_rfc3339();
        }
    }

    combined_excluded_words.sort_unstable();
    combined_excluded_words.dedup();
    params.excluded_words = combined_excluded_words.join("|");

    let query_str = if params.hashtags.is_empty() {
        include_str!("timeline_public_no_hashtags.sql")
    } else {
        include_str!("timeline_public_with_hashtags.sql")
    };

    (query_str, params)
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
    if let Some(conversation) = filters.as_ref().and_then(|f| f.conversation.clone()) {
        return get_thread(
            conn,
            limit,
            min,
            max,
            profile,
            filters,
            Some(conversation),
            uuid,
            id,
        )
        .await
        .map_err(|e| {
            log::error!("{e}");
            e
        });
    }

    if as_id.is_some() || uuid.is_some() || id.is_some() {
        return get_single(conn, limit, min, max, profile, filters, as_id, uuid, id).await;
    }

    if filters.as_ref().and_then(|f| f.username.clone()).is_some() {
        return get_outbox(conn, limit, min, max, profile, filters).await;
    }

    let (query_str, params) =
        build_timeline_query(&filters, limit, min, max, &profile, as_id, uuid, id);

    conn.run(move |c| {
        if params.hashtags.is_empty() {
            // Binding for timeline_public_no_hashtags.sql
            sql_query(query_str)
                .bind::<Text, _>(params.excluded_words)
                .bind::<Bool, _>(params.is_local_view)
                .bind::<Array<Text>, _>(params.to_addresses)
                .bind::<Array<Text>, _>(params.from_addresses)
                .bind::<Text, _>(params.max_date)
                .bind::<Text, _>(params.min_date)
                .bind::<Bool, _>(params.order_asc)
                .bind::<Integer, _>(params.limit)
                .bind::<Text, _>(params.profile_actor_id)
                .load::<CoalescedActivity>(c)
        } else {
            // Binding for timeline_public_with_hashtags.sql
            sql_query(query_str)
                .bind::<Text, _>(params.excluded_words)
                .bind::<Bool, _>(params.is_local_view)
                .bind::<Array<Text>, _>(params.to_addresses)
                .bind::<Array<Text>, _>(params.from_addresses)
                .bind::<Array<Text>, _>(
                    params
                        .hashtags
                        .iter()
                        .map(|h| h.to_lowercase())
                        .collect::<Vec<String>>(),
                )
                .bind::<Text, _>(params.max_date)
                .bind::<Text, _>(params.min_date)
                .bind::<Bool, _>(params.order_asc)
                .bind::<Integer, _>(params.limit)
                .bind::<Text, _>(params.profile_actor_id)
                .load::<CoalescedActivity>(c)
        }
    })
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn get_outbox<C: DbRunner>(
    conn: &C,
    limit: i32,
    min: Option<i64>,
    max: Option<i64>,
    profile: Option<Actor>,
    filters: Option<TimelineFilters>,
) -> Result<Vec<CoalescedActivity>> {
    use diesel::sql_types::{Array, Bool, Integer, Text};

    let query = include_str!("outbox.sql");

    let to_addresses = (*PUBLIC_COLLECTION).clone();
    let filters = filters.ok_or(anyhow!("Outbox query must specify Filters"))?;
    let username = filters
        .username
        .clone()
        .ok_or(anyhow!("Outbox query must specify Username in Filters"))?;

    // Convert ObjectType to lowercase string for SQL, or "NULL" if not specified
    let object_type = filters
        .object_type
        .as_ref()
        .map(|ot| ot.to_string().to_lowercase())
        .unwrap_or("NULL".to_string());

    // Get hashtags from filters
    let hashtags = filters.hashtags.clone();

    let profile_id = profile
        .map(|x| x.id.to_string())
        .unwrap_or("NULL".to_string());

    let min = if let Some(min_val) = min {
        if min_val != 0 {
            if let Some(dt) = DateTime::from_timestamp_micros(min_val) {
                dt.to_rfc3339()
            } else {
                "NULL".to_string()
            }
        } else {
            "NULL".to_string()
        }
    } else {
        "NULL".to_string()
    };

    let max = if let Some(max_val) = max {
        if let Some(dt) = DateTime::from_timestamp_micros(max_val) {
            dt.to_rfc3339()
        } else {
            "NULL".to_string()
        }
    } else {
        "NULL".to_string()
    };

    conn.run(move |c| {
        sql_query(query)
            .bind::<Array<Text>, _>(to_addresses)
            .bind::<Text, _>(username)
            .bind::<Text, _>(max)
            .bind::<Text, _>(min)
            .bind::<Integer, _>(limit)
            .bind::<Text, _>(profile_id)
            .bind::<Bool, _>(false)
            .bind::<Text, _>(object_type)
            .bind::<Array<Text>, _>(hashtags)
            .load::<CoalescedActivity>(c)
    })
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn get_thread<C: DbRunner>(
    conn: &C,
    _limit: i32,
    _min: Option<i64>,
    _max: Option<i64>,
    profile: Option<Actor>,
    _filters: Option<TimelineFilters>,
    as_id: Option<String>,
    _uuid: Option<String>,
    _id: Option<i32>,
) -> Result<Vec<CoalescedActivity>> {
    use diesel::sql_types::{Bool, Integer, Text};

    let query = include_str!("thread.sql");

    let as_id = as_id.ok_or(anyhow!(
        "Object ActivityPub ID must be specified for Thread query"
    ))?;
    let include_descendants = true;
    let include_ancestors = false;
    let include_profile = profile.is_some();
    let profile_id = profile.map(|x| x.id).unwrap_or(-1);

    conn.run(move |c| {
        sql_query(query)
            .bind::<Text, _>(as_id)
            .bind::<Integer, _>(profile_id)
            .bind::<Bool, _>(include_profile)
            .bind::<Bool, _>(include_descendants)
            .bind::<Bool, _>(include_ancestors)
            .load::<CoalescedActivity>(c)
    })
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn get_single<C: DbRunner>(
    conn: &C,
    _limit: i32,
    _min: Option<i64>,
    _max: Option<i64>,
    profile: Option<Actor>,
    _filters: Option<TimelineFilters>,
    as_id: Option<String>,
    uuid: Option<String>,
    id: Option<i32>,
) -> Result<Vec<CoalescedActivity>> {
    use diesel::sql_types::Text;

    let query = include_str!("timeline_single_activity.sql");

    if as_id.is_none() && uuid.is_none() && id.is_none() {
        return Err(anyhow!(
            "ActivityPub ID, UUID, or record ID must be specified"
        ));
    }

    let profile_id = profile.map(|x| x.id);

    let id = if let Some(id) = id {
        id.to_string()
    } else {
        "NULL".to_string()
    };

    let profile_id = if let Some(id) = profile_id {
        id.to_string()
    } else {
        "NULL".to_string()
    };

    conn.run(move |c| {
        sql_query(query)
            .bind::<Nullable<Text>, _>(as_id)
            .bind::<Nullable<Text>, _>(uuid)
            .bind::<Text, _>(id)
            .bind::<Text, _>(profile_id)
            .load::<CoalescedActivity>(c)
    })
    .await
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

pub async fn get_announced<C: DbRunner>(
    conn: &C,
    profile: Actor,
    target_ap_id: String,
) -> Result<Option<String>> {
    conn.run(move |c| {
        activities::table
            .select(activities::ap_id)
            .filter(activities::kind.eq(ActivityType::Announce))
            .filter(activities::revoked.eq(false))
            .filter(activities::target_ap_id.eq(target_ap_id))
            .filter(activities::actor.eq(profile.as_id))
            .order(activities::created_at.desc())
            .get_result(c)
    })
    .await
}

pub async fn get_liked<C: DbRunner>(
    conn: &C,
    profile: Actor,
    target_ap_id: String,
) -> Result<Option<String>> {
    conn.run(move |c| {
        activities::table
            .select(activities::ap_id)
            .filter(activities::kind.eq(ActivityType::Like))
            .filter(activities::revoked.eq(false))
            .filter(activities::target_ap_id.eq(target_ap_id))
            .filter(activities::actor.eq(profile.as_id))
            .order(activities::created_at.desc())
            .get_result(c)
    })
    .await
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
