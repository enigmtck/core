pub mod retriever;
mod types;

use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::models::actors::Actor;
use crate::routes::ActivityJson;
use crate::{Identifier, MaybeMultiple};
use chrono::{DateTime, Utc};
use enum_dispatch::enum_dispatch;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use types::accept::{ApAccept, ApAcceptType};
pub use types::activity::ApActivity;
pub use types::actor::{
    ApActor, ApActorTerse, ApActorType, ApAddress, ApCapabilities, ApPublicKey, PUBLIC_COLLECTION,
};
pub use types::add::{ApAdd, ApAddType};
pub use types::announce::{ApAnnounce, ApAnnounceType};
pub use types::block::{ApBlock, ApBlockType};
pub use types::collection::{
    ActorsPage, ApCollection, ApCollectionType, Collectible, FollowersPage, LeadersPage,
};
pub use types::create::{ApCreate, ApCreateType};
pub use types::delete::{ApDelete, ApDeleteType, ApTombstone};
pub use types::follow::{ApFollow, ApFollowType};
pub use types::like::{ApLike, ApLikeType};
pub use types::note::{ApNote, ApNoteType, Metadata};
pub use types::object::{
    ApBasicContent, ApBasicContentType, ApContext, ApEndpoint, ApHashtag, ApImage, ApImageType,
    ApObject, ApTag, ApTimelineObject,
};

pub use types::question::{ApQuestion, ApQuestionType};

pub use types::attachment::{
    ApAttachment, ApDocument, ApLink, ApProof, ApVerifiableIdentityStatement,
};

//pub use types::remove::{ApRemove, ApRemoveType};
pub use types::session::{ApInstrument, ApInstrumentType, ApSession};
pub use types::undo::{ApUndo, ApUndoType};
pub use types::update::{ApUpdate, ApUpdateType};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ActivityPub {
    Activity(ApActivity),
    Actor(ApActor),
    Object(ApObject),
}

impl FromIterator<ApObject> for Vec<ActivityPub> {
    fn from_iter<I: IntoIterator<Item = ApObject>>(iter: I) -> Self {
        iter.into_iter().map(ActivityPub::from).collect()
    }
}

impl FromIterator<ApActivity> for Vec<ActivityPub> {
    fn from_iter<I: IntoIterator<Item = ApActivity>>(iter: I) -> Self {
        iter.into_iter().map(ActivityPub::from).collect()
    }
}

impl From<ApActivity> for ActivityPub {
    fn from(activity: ApActivity) -> Self {
        ActivityPub::Activity(activity)
    }
}

impl From<&ApActivity> for ActivityPub {
    fn from(activity: &ApActivity) -> Self {
        ActivityPub::Activity(activity.clone())
    }
}

impl From<ApObject> for ActivityPub {
    fn from(object: ApObject) -> Self {
        ActivityPub::Object(object)
    }
}

impl From<&ApObject> for ActivityPub {
    fn from(object: &ApObject) -> Self {
        ActivityPub::Object(object.clone())
    }
}

impl From<ApActor> for ActivityPub {
    fn from(actor: ApActor) -> Self {
        ActivityPub::Actor(actor)
    }
}

impl ActivityPub {
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            ActivityPub::Object(object) => object.timestamp(),
            ActivityPub::Activity(activity) => match activity {
                ApActivity::Create(create) => create
                    .ephemeral
                    .clone()
                    .and_then(|x| x.created_at)
                    .unwrap_or(Utc::now()),
                ApActivity::Announce(announce) => announce
                    .ephemeral
                    .clone()
                    .and_then(|x| x.created_at)
                    .unwrap_or(Utc::now()),
                _ => Utc::now(),
            },
            _ => Utc::now(),
        }
    }

    pub fn time(timestamp: DateTime<Utc>) -> String {
        timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}

pub trait Temporal {
    fn published(&self) -> String;
    fn created_at(&self) -> Option<DateTime<Utc>>;
    fn updated_at(&self) -> Option<DateTime<Utc>>;
}

#[enum_dispatch(ApActivity)]
pub trait Inbox {
    async fn inbox(&self, conn: Db, channels: EventChannels, raw: Value) -> Result<Status, Status>;
    fn actor(&self) -> ApAddress;
}

#[enum_dispatch(ApActivity, ApObject)]
pub trait Outbox {
    async fn outbox(
        &self,
        conn: Db,
        events: EventChannels,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status>;
}
