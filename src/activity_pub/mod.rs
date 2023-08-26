pub mod retriever;
pub mod sender;
mod types;

use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::fairings::faktory::FaktoryConnection;
use crate::models::profiles::Profile;
use chrono::{DateTime, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
pub use types::accept::{ApAccept, ApAcceptType};
pub use types::activity::ApActivity;
pub use types::actor::{ApActor, ApActorType, ApAddress, ApPublicKey};
pub use types::add::{ApAdd, ApAddType};
pub use types::announce::{ApAnnounce, ApAnnounceType};
pub use types::block::{ApBlock, ApBlockType};
pub use types::collection::{
    ActivitiesPage, ActorsPage, ApCollection, ApCollectionPage, ApCollectionPageType,
    ApCollectionType, FollowersPage, IdentifiedVaultItems, LeadersPage,
};
pub use types::create::{ApCreate, ApCreateType};
pub use types::delete::{ApDelete, ApDeleteType};
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
    fn created_at(&self) -> Option<DateTime<Utc>>;
    fn updated_at(&self) -> Option<DateTime<Utc>>;
}

pub trait Inbox {
    async fn inbox(&self, conn: Db, faktory: FaktoryConnection) -> Result<Status, Status>;
}

pub trait Outbox {
    async fn outbox(
        &self,
        conn: Db,
        faktory: FaktoryConnection,
        events: EventChannels,
        profile: Profile,
    ) -> Result<String, Status>;
}
