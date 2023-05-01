use crate::{
    activity_pub::{
        ApAccept, ApAdd, ApAnnounce, ApBlock, ApCreate, ApDelete, ApInvite, ApJoin, ApLike, ApNote,
        ApRemove, ApUndo, ApUpdate,
    },
    models::activities::ExtendedActivity,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use super::follow::ApFollow;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ApActivity {
    Delete(Box<ApDelete>),
    Like(Box<ApLike>),
    Undo(Box<ApUndo>),
    Accept(Box<ApAccept>),
    Follow(ApFollow),
    Announce(ApAnnounce),
    Create(ApCreate),
    Invite(ApInvite),
    Join(ApJoin),
    Update(ApUpdate),
    Block(ApBlock),
    Add(ApAdd),
    Remove(ApRemove),
}

impl TryFrom<ExtendedActivity> for ApActivity {
    type Error = &'static str;

    fn try_from(
        (activity, note, remote_note, profile): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        match activity.kind.as_str() {
            "Create" if note.is_some() => Ok(ApActivity::Create(ApCreate::from(ApNote::from(
                note.unwrap(),
            )))),
            "Delete" if note.is_some() => {
                if let Ok(delete) = ApDelete::try_from(ApNote::from(note.unwrap())) {
                    Ok(ApActivity::Delete(Box::new(delete)))
                } else {
                    Err("")
                }
            }
            _ => Err(""),
        }
    }
}
