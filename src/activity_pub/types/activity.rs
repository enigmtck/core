use crate::{
    activity_pub::{
        ApAccept, ApAdd, ApAnnounce, ApBlock, ApCreate, ApDelete, ApInvite, ApJoin, ApLike, ApNote,
        ApRemove, ApUndo, ApUpdate,
    },
    models::activities::{ActivityType, ExtendedActivity},
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
        (activity, note, remote_note, profile, remote_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        match activity.kind {
            ActivityType::Create if note.is_some() => Ok(ApActivity::Create(ApCreate::from(
                ApNote::from(note.unwrap()),
            ))),
            ActivityType::Announce if (note.is_some() | remote_note.is_some()) => {
                if let Ok(activity) =
                    ApAnnounce::try_from((activity, note, remote_note, profile, remote_actor))
                {
                    Ok(ApActivity::Announce(activity))
                } else {
                    Err("")
                }
            }
            ActivityType::Like if (note.is_some() | remote_note.is_some()) => {
                if let Ok(activity) =
                    ApLike::try_from((activity, note, remote_note, profile, remote_actor))
                {
                    Ok(ApActivity::Like(Box::new(activity)))
                } else {
                    Err("")
                }
            }
            ActivityType::Delete if note.is_some() => {
                if let Ok(delete) = ApDelete::try_from(ApNote::from(note.unwrap())) {
                    Ok(ApActivity::Delete(Box::new(delete)))
                } else {
                    Err("")
                }
            }
            ActivityType::Follow if (profile.is_some() | remote_actor.is_some()) => {
                if let Ok(follow) =
                    ApFollow::try_from((activity, note, remote_note, profile, remote_actor))
                {
                    Ok(ApActivity::Follow(follow))
                } else {
                    Err("")
                }
            }
            _ => Err(""),
        }
    }
}
