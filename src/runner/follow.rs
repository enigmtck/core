use diesel::prelude::*;
use faktory::Job;
use std::io;
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{sender::send_activity, ApAccept, ApActivity, ApAddress, ApFollow, ApUndo},
    models::{
        followers::{Follower, NewFollower},
        follows::Follow,
        leaders::{Leader, NewLeader},
    },
    runner::{
        activity::{get_activity_by_uuid, get_remote_activity_by_apid},
        actor::get_actor,
        get_inboxes, send_to_inboxes,
        user::{get_profile, get_profile_by_ap_id},
    },
    schema::{followers, follows, leaders},
    MaybeReference,
};

use super::POOL;

pub fn get_leader_by_actor_ap_id_and_profile(ap_id: String, profile_id: i32) -> Option<Leader> {
    if let Ok(mut conn) = POOL.get() {
        match leaders::table
            .filter(leaders::leader_ap_id.eq(ap_id))
            .filter(leaders::profile_id.eq(profile_id))
            .first::<Leader>(&mut conn)
        {
            Ok(x) => Option::from(x),
            Err(_) => Option::None,
        }
    } else {
        Option::None
    }
}

pub fn get_follow_by_uuid(uuid: String) -> Option<Follow> {
    if let Ok(mut conn) = POOL.get() {
        match follows::table
            .filter(follows::uuid.eq(uuid))
            .first::<Follow>(&mut conn)
        {
            Ok(x) => Option::from(x),
            Err(_) => Option::None,
        }
    } else {
        None
    }
}

pub fn process_follow(job: Job) -> io::Result<()> {
    log::debug!("PROCESSING OUTGOING FOLLOW REQUEST");

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();
        log::debug!("LOOKING FOR UUID {uuid}");

        if let Some((
            activity,
            target_note,
            target_remote_note,
            target_profile,
            target_remote_actor,
        )) = get_activity_by_uuid(uuid.clone())
        {
            log::debug!("FOUND ACTIVITY\n{activity:#?}");
            if let Some(sender) = get_profile(activity.profile_id) {
                if let Ok(activity) = ApActivity::try_from((
                    activity,
                    target_note,
                    target_remote_note,
                    target_profile,
                    target_remote_actor,
                )) {
                    let inboxes: Vec<ApAddress> = get_inboxes(activity.clone(), sender.clone());
                    send_to_inboxes(inboxes, sender, activity.clone());
                }
            }
        }
    }

    Ok(())
}

pub fn create_leader(leader: NewLeader) -> Option<Leader> {
    if let Ok(mut conn) = POOL.get() {
        match diesel::insert_into(leaders::table)
            .values(&leader)
            .get_result::<Leader>(&mut conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn process_accept(job: Job) -> io::Result<()> {
    log::debug!("PROCESSING INCOMING ACCEPT REQUEST");

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();
        log::debug!("APID: {ap_id}");

        if let Some(activity) = get_remote_activity_by_apid(ap_id) {
            if let Ok(accept) = ApAccept::try_from(activity) {
                if let MaybeReference::Actual(ApActivity::Follow(follow)) = accept.object.clone() {
                    if let Some(profile) = get_profile_by_ap_id(follow.actor.to_string()) {
                        if let Ok(mut leader) = NewLeader::try_from(accept.clone()) {
                            leader.link(profile);

                            if let Some(leader) = create_leader(leader) {
                                log::debug!("LEADER CREATED: {}", leader.uuid);
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

pub fn delete_leader_by_ap_id_and_profile_id(
    ap_id: String,
    profile_id: i32,
) -> Result<usize, DeleteLeaderError> {
    if let Ok(mut conn) = POOL.get() {
        match diesel::delete(
            leaders::table
                .filter(leaders::leader_ap_id.eq(ap_id))
                .filter(leaders::profile_id.eq(profile_id)),
        )
        .execute(&mut conn)
        {
            Ok(x) => Ok(x),
            Err(e) => {
                log::error!("FAILED TO DELETE LEADER\n{e:#?}");
                Err(DeleteLeaderError::DatabaseError(e))
            }
        }
    } else {
        Err(DeleteLeaderError::ConnectionError)
    }
}

pub fn process_undo_follow(job: Job) -> io::Result<()> {
    log::debug!("PROCESSING FOLLOW UNDO REQUEST");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();
        log::debug!("UUID: {uuid}");

        if let Some(follow) = get_follow_by_uuid(uuid) {
            let follow: ApFollow = follow.into();
            let undo: ApUndo = follow.clone().into();

            log::debug!("UNDO\n{undo:#?}");

            if let Some(profile) = get_profile_by_ap_id(follow.actor.to_string()) {
                if let MaybeReference::Reference(to) = follow.object.clone() {
                    handle.block_on(async {
                        if let Some((actor, _)) = get_actor(profile.clone(), to.clone()).await {
                            let inbox = actor.inbox;

                            match send_activity(
                                ApActivity::Undo(Box::new(undo)),
                                profile.clone(),
                                inbox.clone(),
                            )
                            .await
                            {
                                Ok(_) => {
                                    log::info!("UNDO SENT: {inbox:#?}");
                                    if delete_leader_by_ap_id_and_profile_id(to, profile.id).is_ok()
                                    {
                                        log::info!("LEADER DELETED");
                                    }
                                }
                                Err(e) => log::error!("ERROR SENDING UNDO REQUEST: {e:#?}"),
                            }
                        }
                    })
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
pub enum DeleteFollowerError {
    ConnectionError,
    DatabaseError(diesel::result::Error),
}

pub fn delete_follower_by_ap_id(ap_id: String) -> Result<usize, DeleteFollowerError> {
    if let Ok(mut conn) = POOL.get() {
        match diesel::delete(followers::table.filter(followers::ap_id.eq(ap_id))).execute(&mut conn)
        {
            Ok(x) => Ok(x),
            Err(e) => {
                log::error!("FAILED TO DELETE FOLLOWER\n{e:#?}");
                Err(DeleteFollowerError::DatabaseError(e))
            }
        }
    } else {
        Err(DeleteFollowerError::ConnectionError)
    }
}

pub fn process_remote_undo_follow(job: Job) -> io::Result<()> {
    log::debug!("PROCESSING INCOMING UNDO FOLLOW REQUEST");

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();
        log::debug!("APID: {ap_id}");

        if delete_follower_by_ap_id(ap_id.clone()).is_ok() {
            log::info!("FOLLOWER RECORD DELETED: {ap_id}");
        }
    }

    Ok(())
}

pub fn acknowledge_followers(job: Job) -> io::Result<()> {
    log::debug!("running acknowledge job");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    log::debug!("PROCESSING INCOMING ACCEPT REQUEST");

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();
        log::debug!("APID: {ap_id}");

        handle.block_on(async {
            if let Some(activity) = get_remote_activity_by_apid(ap_id) {
                if let Ok(follow) = ApFollow::try_from(activity) {
                    if let Ok(accept) = ApAccept::try_from(follow.clone()) {
                        if let Some(profile) =
                            get_profile_by_ap_id(accept.actor.clone().to_string())
                        {
                            if let Some((actor, _)) =
                                get_actor(profile.clone(), follow.actor.clone().to_string()).await
                            {
                                let inbox = actor.inbox;

                                match send_activity(
                                    ApActivity::Accept(Box::new(accept)),
                                    profile.clone(),
                                    inbox.clone(),
                                )
                                .await
                                {
                                    Ok(_) => {
                                        log::info!("ACCEPT SENT: {inbox:#?}");

                                        if let Ok(mut follower) = NewFollower::try_from(follow) {
                                            follower.link(profile.clone());

                                            log::debug!("NEW FOLLOWER\n{follower:#?}");
                                            if create_follower(follower).is_some() {
                                                log::info!("FOLLOWER CREATED");
                                            }
                                        }
                                    }
                                    Err(e) => log::error!("ERROR SENDING UNDO REQUEST: {e:#?}"),
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    Ok(())
}

pub fn create_follower(follower: NewFollower) -> Option<Follower> {
    if let Ok(mut conn) = POOL.get() {
        match diesel::insert_into(followers::table)
            .values(&follower)
            .get_result::<Follower>(&mut conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}
