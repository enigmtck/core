pub mod retriever;
pub mod sender;
mod types;

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
    ApAttachment, ApAttachmentType, ApBasicContent, ApBasicContentType, ApContext, ApEndpoint,
    ApImage, ApImageType, ApObject, ApTag,
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
