use crate::{
    activity_pub::{
        ApAccept, ApAdd, ApAnnounce, ApBlock, ApCreate, ApDelete, ApLike, ApNote, ApRemove, ApUndo,
        ApUpdate,
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
    //Invite(ApInvite),
    //Join(ApJoin),
    Update(ApUpdate),
    Block(ApBlock),
    Add(ApAdd),
    Remove(ApRemove),
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
            //ApActivity::Invite(invite) => invite.id.clone(),
            //ApActivity::Join(join) => join.id.clone(),
            ApActivity::Update(update) => update.id.clone(),
            ApActivity::Block(block) => block.id.clone(),
            _ => None,
        }
    }
}

impl TryFrom<CoalescedActivity> for ApActivity {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        match coalesced.kind {
            ActivityType::Create => ApCreate::try_from(coalesced).map(ApActivity::Create),
            ActivityType::Announce => ApAnnounce::try_from(coalesced).map(ApActivity::Announce),
            _ => {
                log::error!("Failed to match implemented activity\n{coalesced:#?}");
                Err(anyhow!("Failed to match implemented activity"))
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
            ActivityType::Create => {
                ApCreate::try_from((activity, target_activity, target_object, target_actor))
                    .map(ApActivity::Create)
            }
            ActivityType::Announce => {
                ApAnnounce::try_from((activity, target_activity, target_object, target_actor))
                    .map(ApActivity::Announce)
            }
            ActivityType::Like => {
                ApLike::try_from((activity, target_activity, target_object, target_actor))
                    .map(|activity| ApActivity::Like(Box::new(activity)))
            }
            ActivityType::Delete => {
                let note = ApNote::try_from(target_object.unwrap())?;
                ApDelete::try_from(note).map(|mut delete| {
                    delete.id = activity.ap_id;
                    ApActivity::Delete(Box::new(delete))
                })
            }
            ActivityType::Follow => {
                ApFollow::try_from((activity, target_activity, target_object, target_actor))
                    .map(ApActivity::Follow)
            }
            ActivityType::Undo => {
                ApUndo::try_from((activity, target_activity, target_object, target_actor))
                    .map(|undo| ApActivity::Undo(Box::new(undo)))
            }
            ActivityType::Accept => {
                ApAccept::try_from((activity, target_activity, target_object, target_actor))
                    .map(|accept| ApActivity::Accept(Box::new(accept)))
            }
            _ => {
                log::error!(
                    "Failed to match implemented activity in TryFrom for ApActivity\nACTIVITY: {activity:#?}\nTARGET_ACTIVITY: {target_activity:#?}\nTARGET_OBJECT: {target_object:#?}\nTARGET_ACTOR {target_actor:#?}"
                );
                Err(anyhow!("Failed to match implemented activity"))
            }
        }
    }
}
