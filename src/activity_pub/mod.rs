pub mod retriever;
pub mod sender;
mod types;

use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::models::profiles::Profile;
use crate::{Identifier, MaybeMultiple};
use chrono::{DateTime, NaiveDateTime, Utc};
use enum_dispatch::enum_dispatch;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use types::accept::{ApAccept, ApAcceptType};
pub use types::activity::ApActivity;
pub use types::actor::{ApActor, ApActorType, ApAddress, ApPublicKey};
pub use types::add::{ApAdd, ApAddType};
pub use types::announce::{ApAnnounce, ApAnnounceType};
pub use types::block::{ApBlock, ApBlockType};
pub use types::collection::{
    ActivitiesPage, ActorsPage, ApCollection, ApCollectionPage, ApCollectionPageType,
    ApCollectionType, Collectible, FollowersPage, IdentifiedVaultItems, LeadersPage,
};
pub use types::create::{ApCreate, ApCreateType};
pub use types::delete::{ApDelete, ApDeleteType, ApTombstone};
pub use types::follow::{ApFollow, ApFollowType};
pub use types::invite::{ApInvite, ApInviteType};
pub use types::join::{ApJoin, ApJoinType};
pub use types::like::{ApLike, ApLikeType};
pub use types::note::{ApNote, ApNoteType, FullyQualifiedTimelineItem, Metadata};
pub use types::object::{
    ApBasicContent, ApBasicContentType, ApContext, ApEndpoint, ApImage, ApImageType, ApObject,
    ApTag,
};

pub use types::attachment::{
    ApAttachment, ApDocument, ApLink, ApProof, ApVerifiableIdentityStatement,
};

pub use types::remove::{ApRemove, ApRemoveType};
pub use types::session::JoinData;
pub use types::session::{ApInstrument, ApInstrumentType, ApInstruments, ApSession};
pub use types::undo::{ApUndo, ApUndoType};
pub use types::update::{ApUpdate, ApUpdateType};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ActivityPub {
    Activity(ApActivity),
    Actor(ApActor),
    Object(ApObject),
}

pub trait Temporal {
    fn published(&self) -> String;
    fn created_at(&self) -> Option<NaiveDateTime>;
    fn updated_at(&self) -> Option<NaiveDateTime>;
}

#[enum_dispatch(ApActivity)]
pub trait Inbox {
    async fn inbox(&self, conn: Db, channels: EventChannels, raw: Value) -> Result<Status, Status>;
}

#[enum_dispatch(ApActivity, ApObject)]
pub trait Outbox {
    async fn outbox(
        &self,
        conn: Db,
        events: EventChannels,
        profile: Profile,
    ) -> Result<String, Status>;
}
