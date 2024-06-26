use crate::{
    activity_pub::{
        ApAccept, ApAdd, ApAnnounce, ApBlock, ApCreate, ApDelete, ApInvite, ApJoin, ApLike, ApNote,
        ApRemove, ApUndo, ApUpdate,
    },
    models::activities::ExtendedActivity,
};
use anyhow::anyhow;
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

impl TryFrom<RecursiveActivity> for ApActivity {
    type Error = anyhow::Error;

    fn try_from(
        ((activity, note, remote_note, profile, remote_actor, remote_question), recursive): RecursiveActivity,
    ) -> Result<Self, Self::Error> {
        match activity.kind.to_string().to_lowercase().as_str() {
            "create" if note.is_some() => ApCreate::try_from((
                activity,
                note,
                remote_note,
                profile,
                remote_actor,
                remote_question,
            ))
            .map(ApActivity::Create),
            "announce" if note.is_some() || remote_note.is_some() => ApAnnounce::try_from((
                activity,
                note,
                remote_note,
                profile,
                remote_actor,
                remote_question,
            ))
            .map(ApActivity::Announce),
            "like" if note.is_some() || remote_note.is_some() => ApLike::try_from((
                activity,
                note,
                remote_note,
                profile,
                remote_actor,
                remote_question,
            ))
            .map(|activity| ApActivity::Like(Box::new(activity))),
            "delete" if note.is_some() => ApDelete::try_from(ApNote::from(note.unwrap()))
                .map(|delete| ApActivity::Delete(Box::new(delete))),
            "follow" if profile.is_some() || remote_actor.is_some() => ApFollow::try_from((
                activity,
                note,
                remote_note,
                profile,
                remote_actor,
                remote_question,
            ))
            .map(ApActivity::Follow),
            "undo" if recursive.is_some() => ApUndo::try_from((
                (
                    activity,
                    note,
                    remote_note,
                    profile,
                    remote_actor,
                    remote_question,
                ),
                recursive,
            ))
            .map(|undo| ApActivity::Undo(Box::new(undo))),
            "accept" if recursive.is_some() => ApAccept::try_from((
                (
                    activity,
                    note,
                    remote_note,
                    profile,
                    remote_actor,
                    remote_question,
                ),
                recursive,
            ))
            .map(|accept| ApActivity::Accept(Box::new(accept))),
            _ => {
                log::error!("FAILED TO MATCH IMPLEMENTED ACTIVITY\n{activity:#?}");
                Err(anyhow!("FAILED TO MATCH IMPLEMENTED ACTIVITY"))
            }
        }
    }
}
