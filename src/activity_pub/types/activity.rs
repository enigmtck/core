use crate::{
    activity_pub::{
        ApAccept, ApAdd, ApAnnounce, ApBlock, ApCreate, ApDelete, ApInvite, ApJoin, ApLike, ApNote,
        ApRemove, ApUndo, ApUpdate,
    },
    models::activities::ExtendedActivity,
};
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use super::follow::ApFollow;

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
    Invite(ApInvite),
    Join(ApJoin),
    Update(ApUpdate),
    Block(ApBlock),
    Add(ApAdd),
    Remove(ApRemove),
}

pub type RecursiveActivity = (ExtendedActivity, Option<ExtendedActivity>);

cfg_if::cfg_if! {
    if #[cfg(feature = "pg" )] {
        impl TryFrom<RecursiveActivity> for ApActivity {
            type Error = &'static str;

            fn try_from(
                ((activity, note, remote_note, profile, remote_actor), recursive): RecursiveActivity,
            ) -> Result<Self, Self::Error> {
                use crate::models::activities::ActivityType;

                match activity.kind {
                    ActivityType::Create if note.is_some() => {
                        ApCreate::try_from((activity, note, remote_note, profile, remote_actor))
                            .map(ApActivity::Create)
                    }
                    ActivityType::Announce if note.is_some() || remote_note.is_some() => {
                        ApAnnounce::try_from((activity, note, remote_note, profile, remote_actor))
                            .map(ApActivity::Announce)
                    }
                    ActivityType::Like if note.is_some() || remote_note.is_some() => {
                        ApLike::try_from((activity, note, remote_note, profile, remote_actor))
                            .map(|activity| ApActivity::Like(Box::new(activity)))
                    }
                    ActivityType::Delete if note.is_some() => {
                        ApDelete::try_from(ApNote::from(note.unwrap()))
                            .map(|delete| ApActivity::Delete(Box::new(delete)))
                    }
                    ActivityType::Follow if profile.is_some() || remote_actor.is_some() => {
                        ApFollow::try_from((activity, note, remote_note, profile, remote_actor))
                            .map(ApActivity::Follow)
                    }
                    ActivityType::Undo if recursive.is_some() => ApUndo::try_from((
                        (activity, note, remote_note, profile, remote_actor),
                        recursive,
                    ))
                        .map(|undo| ApActivity::Undo(Box::new(undo))),
                    ActivityType::Accept if recursive.is_some() => ApAccept::try_from((
                        (activity, note, remote_note, profile, remote_actor),
                        recursive,
                    ))
                        .map(|accept| ApActivity::Accept(Box::new(accept))),
                    _ => {
                        log::error!("FAILED TO MATCH IMPLEMENTED ACTIVITY\n{activity:#?}");
                        Err("FAILED TO MATCH IMPLEMENTED ACTIVITY")
                    }
                }
            }
        }
    } else if #[cfg(feature = "sqlite" )] {
        impl TryFrom<RecursiveActivity> for ApActivity {
            type Error = &'static str;

            fn try_from(
                ((activity, note, remote_note, profile, remote_actor), recursive): RecursiveActivity,
            ) -> Result<Self, Self::Error> {
                match activity.kind.to_lowercase().as_str() {
                    "create" if note.is_some() => {
                        ApCreate::try_from((activity, note, remote_note, profile, remote_actor))
                            .map(ApActivity::Create)
                    }
                    "announce" if note.is_some() || remote_note.is_some() => {
                        ApAnnounce::try_from((activity, note, remote_note, profile, remote_actor))
                            .map(ApActivity::Announce)
                    }
                    "like" if note.is_some() || remote_note.is_some() => {
                        ApLike::try_from((activity, note, remote_note, profile, remote_actor))
                            .map(|activity| ApActivity::Like(Box::new(activity)))
                    }
                    "delete" if note.is_some() => ApDelete::try_from(ApNote::from(note.unwrap()))
                        .map(|delete| ApActivity::Delete(Box::new(delete))),
                    "follow" if profile.is_some() || remote_actor.is_some() => {
                        ApFollow::try_from((activity, note, remote_note, profile, remote_actor))
                            .map(ApActivity::Follow)
                    }
                    "undo" if recursive.is_some() => ApUndo::try_from((
                        (activity, note, remote_note, profile, remote_actor),
                        recursive,
                    ))
                        .map(|undo| ApActivity::Undo(Box::new(undo))),
                    "accept" if recursive.is_some() => ApAccept::try_from((
                        (activity, note, remote_note, profile, remote_actor),
                        recursive,
                    ))
                        .map(|accept| ApActivity::Accept(Box::new(accept))),
                    _ => {
                        log::error!("FAILED TO MATCH IMPLEMENTED ACTIVITY\n{activity:#?}");
                        Err("FAILED TO MATCH IMPLEMENTED ACTIVITY")
                    }
                }
            }
        }
    }
}
