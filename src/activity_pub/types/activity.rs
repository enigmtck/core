use crate::{
    activity_pub::{
        ApAccept,
        ApAdd,
        ApAnnounce,
        ApBlock,
        ApCreate,
        ApDelete,
        ApLike,
        //ApRemove,
        ApUndo,
        ApUpdate,
    },
    db::Db,
    MaybeReference,
};
use anyhow::Result;
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;

use super::{follow::ApFollow, object::ApObject};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[enum_dispatch]
#[serde(untagged)]
pub enum ApActivity {
    Delete(Box<ApDelete>),
    Like(Box<ApLike>),
    Undo(Box<ApUndo>),
    Accept(Box<ApAccept>),
    Follow(ApFollow),
    Announce(ApAnnounce),
    Create(ApCreate),
    Update(ApUpdate),
    Block(ApBlock),
    Add(ApAdd),
    //Remove(ApRemove),
}

impl ApActivity {
    pub fn as_id(&self) -> Option<String> {
        match self {
            ApActivity::Like(like) => like.id.clone(),
            ApActivity::Follow(follow) => follow.id.clone(),
            ApActivity::Announce(announce) => announce.id.clone(),
            ApActivity::Delete(delete) => delete.id.clone(),
            ApActivity::Undo(undo) => undo.id.clone(),
            ApActivity::Accept(accept) => accept.id.clone(),
            ApActivity::Create(create) => create.id.clone(),
            ApActivity::Update(update) => update.id.clone(),
            ApActivity::Block(block) => block.id.clone(),
            _ => None,
        }
    }

    pub fn formalize(&self) -> Self {
        match self.clone() {
            ApActivity::Announce(mut announce) => {
                announce.ephemeral = None;
                if let MaybeReference::Actual(ApObject::Note(ref mut note)) = announce.object {
                    note.context = None;
                    note.ephemeral = None;
                    note.instrument = None;
                }
                announce.into()
            }
            ApActivity::Create(mut create) => {
                create.ephemeral = None;
                if let MaybeReference::Actual(ApObject::Note(ref mut note)) = create.object {
                    note.context = None;
                    note.ephemeral = None;
                    note.instrument = None;
                }
                create.into()
            }
            _ => self.clone(),
        }
    }

    pub async fn load_ephemeral(&self, conn: &Db) -> Self {
        match self.clone() {
            ApActivity::Create(mut create) => {
                if let MaybeReference::Actual(ApObject::Note(ref mut note)) = create.object {
                    note.load_ephemeral(conn).await;
                }
                create.into()
            }
            _ => self.clone(),
        }
    }
}

impl TryFrom<Value> for ApActivity {
    type Error = anyhow::Error;

    fn try_from(value: Value) -> Result<ApActivity> {
        serde_json::from_value(value).map_err(anyhow::Error::msg)
    }
}
