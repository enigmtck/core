use faktory::Job;
use std::io;
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{sender::send_activity, ApAccept, ApActivity, ApAddress, ApFollow},
    models::{
        activities::{get_activity, get_activity_by_uuid},
        followers::{create_follower, delete_follower_by_ap_id, NewFollower},
        //follows::Follow,
        leaders::{create_leader, NewLeader},
        profiles::{get_profile, get_profile_by_ap_id},
    },
    runner::{actor::get_actor, get_inboxes, send_to_inboxes},
    MaybeReference,
};

pub fn process_follow(job: Job) -> io::Result<()> {
    log::debug!("PROCESSING OUTGOING FOLLOW REQUEST");

    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();
        log::debug!("LOOKING FOR UUID {uuid}");

        if let Some((
            activity,
            target_note,
            target_remote_note,
            target_profile,
            target_remote_actor,
        )) = handle.block_on(async { get_activity_by_uuid(None, uuid.clone()).await })
        {
            log::debug!("FOUND ACTIVITY\n{activity:#?}");
            if let Some(profile_id) = activity.profile_id {
                if let Some(sender) = handle.block_on(async { get_profile(None, profile_id).await })
                {
                    if let Ok(activity) = ApActivity::try_from((
                        (
                            activity,
                            target_note,
                            target_remote_note,
                            target_profile,
                            target_remote_actor,
                        ),
                        None,
                    )) {
                        let inboxes: Vec<ApAddress> = handle.block_on(async {
                            get_inboxes(activity.clone(), sender.clone()).await
                        });
                        handle.block_on(async {
                            send_to_inboxes(inboxes, sender, activity.clone()).await
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn process_accept(job: Job) -> io::Result<()> {
    log::debug!("PROCESSING INCOMING ACCEPT REQUEST");

    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();
        log::debug!("UUID: {uuid}");

        if let Some(extended_accept) =
            handle.block_on(async { get_activity_by_uuid(None, uuid).await })
        {
            if let Some(follow_id) = extended_accept.0.target_activity_id {
                if let Some(extended_follow) =
                    handle.block_on(async { get_activity(None, follow_id).await })
                {
                    if let Ok(ApActivity::Accept(accept)) =
                        (extended_accept, Some(extended_follow)).try_into()
                    {
                        if let MaybeReference::Actual(ApActivity::Follow(follow)) =
                            accept.object.clone()
                        {
                            if let Some(profile) = handle.block_on(async {
                                get_profile_by_ap_id(None, follow.actor.to_string()).await
                            }) {
                                if let Ok(mut leader) = NewLeader::try_from(*accept.clone()) {
                                    leader.link(profile);

                                    if let Some(leader) =
                                        handle.block_on(async { create_leader(None, leader).await })
                                    {
                                        log::debug!("LEADER CREATED: {}", leader.uuid);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
pub enum DeleteLeaderError {
    ConnectionError,
    DatabaseError(diesel::result::Error),
}

#[derive(Debug)]
pub enum DeleteFollowerError {
    ConnectionError,
    DatabaseError(diesel::result::Error),
}

pub fn process_remote_undo_follow(job: Job) -> io::Result<()> {
    log::debug!("PROCESSING INCOMING UNDO FOLLOW REQUEST");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();
        log::debug!("APID: {ap_id}");

        if handle.block_on(async { delete_follower_by_ap_id(None, ap_id.clone()).await }) {
            log::info!("FOLLOWER RECORD DELETED: {ap_id}");
        }
    }

    Ok(())
}

pub fn acknowledge_followers(job: Job) -> io::Result<()> {
    log::debug!("running acknowledge job");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    log::debug!("PROCESSING INCOMING FOLLOW REQUEST");

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();
        log::debug!("UUID: {uuid}");

        if let Some(extended_follow) =
            handle.block_on(async { get_activity_by_uuid(None, uuid).await })
        {
            if let Ok(follow) = ApFollow::try_from(extended_follow) {
                if let Ok(accept) = ApAccept::try_from(follow.clone()) {
                    if let Some(profile) = handle.block_on(async {
                        get_profile_by_ap_id(None, accept.actor.clone().to_string()).await
                    }) {
                        if let Some((actor, _)) = handle.block_on(async {
                            get_actor(profile.clone(), follow.actor.clone().to_string()).await
                        }) {
                            let inbox = actor.inbox;

                            match handle.block_on(async {
                                send_activity(
                                    ApActivity::Accept(Box::new(accept)),
                                    profile.clone(),
                                    inbox.clone(),
                                )
                                .await
                            }) {
                                Ok(_) => {
                                    log::info!("ACCEPT SENT: {inbox:#?}");

                                    if let Ok(mut follower) = NewFollower::try_from(follow) {
                                        follower.link(profile.clone());

                                        log::debug!("NEW FOLLOWER\n{follower:#?}");
                                        if handle
                                            .block_on(async {
                                                create_follower(None, follower).await
                                            })
                                            .is_some()
                                        {
                                            log::info!("FOLLOWER CREATED");
                                        }
                                    }
                                }
                                Err(e) => log::error!("ERROR SENDING ACCEPT: {e:#?}"),
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
