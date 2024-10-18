use crate::{
    activity_pub::{
        ApAccept, ApAdd, ApAnnounce, ApBlock, ApCreate, ApDelete, ApInvite, ApJoin, ApLike, ApNote,
        ApRemove, ApUndo, ApUpdate,
    },
    models::{
        activities::{ActivityType, ExtendedActivity},
        pg::coalesced_activity::CoalescedActivity,
    },
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

//pub type RecursiveActivity = (ExtendedActivity, Option<ExtendedActivity>);

// impl TryFrom<ExtendedActivity> for ApActivity {
//     type Error = anyhow::Error;

//     fn try_from(activity: ExtendedActivity) -> Result<Self, Self::Error> {
//         ApActivity::try_from((activity, None))
//     }
// }

impl TryFrom<CoalescedActivity> for ApActivity {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        match coalesced.kind {
            ActivityType::Create => ApCreate::try_from(coalesced).map(ApActivity::Create),
            ActivityType::Announce => ApAnnounce::try_from(coalesced).map(ApActivity::Announce),
            _ => {
                log::error!("FAILED TO MATCH IMPLEMENTED ACTIVITY\n{coalesced:#?}");
                Err(anyhow!("FAILED TO MATCH IMPLEMENTED ACTIVITY"))
            }
        }
    }
}

impl TryFrom<ExtendedActivity> for ApActivity {
    type Error = anyhow::Error;

    fn try_from(
        (activity, target_activity, target_object, target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        match activity.kind {
            ActivityType::Create if target_object.is_some() => {
                ApCreate::try_from((activity, target_activity, target_object, target_actor))
                    .map(ApActivity::Create)
            }
            ActivityType::Announce if target_object.is_some() => {
                ApAnnounce::try_from((activity, target_activity, target_object, target_actor))
                    .map(ApActivity::Announce)
            }
            ActivityType::Like if target_object.is_some() => {
                ApLike::try_from((activity, target_activity, target_object, target_actor))
                    .map(|activity| ApActivity::Like(Box::new(activity)))
            }
            ActivityType::Delete if target_object.is_some() => {
                ApDelete::try_from(ApNote::try_from(target_object.unwrap())?)
                    .map(|delete| ApActivity::Delete(Box::new(delete)))
            }
            ActivityType::Follow if target_actor.is_some() => {
                ApFollow::try_from((activity, target_activity, target_object, target_actor))
                    .map(ApActivity::Follow)
            }
            ActivityType::Undo if target_activity.is_some() => {
                ApUndo::try_from((activity, target_activity, target_object, target_actor))
                    .map(|undo| ApActivity::Undo(Box::new(undo)))
            }
            ActivityType::Accept if target_activity.is_some() => {
                ApAccept::try_from((activity, target_activity, target_object, target_actor))
                    .map(|accept| ApActivity::Accept(Box::new(accept)))
            }
            _ => {
                log::error!("FAILED TO MATCH IMPLEMENTED ACTIVITY\n{activity:#?}");
                Err(anyhow!("FAILED TO MATCH IMPLEMENTED ACTIVITY"))
            }
        }
    }
}
