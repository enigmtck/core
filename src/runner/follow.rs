use anyhow::Result;
use faktory::Job;
use std::io;
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{sender::send_activity, ApAccept, ApActivity, ApAddress, ApFollow},
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::{get_activity, get_activity_by_uuid},
        followers::{create_follower, delete_follower_by_ap_id, NewFollower},
        //follows::Follow,
        leaders::{create_leader, NewLeader},
        profiles::{get_profile, get_profile_by_ap_id},
    },
    runner::{actor::get_actor, get_inboxes, send_to_inboxes, TaskError},
    MaybeReference,
};

pub async fn process_follow_task(
    conn: Option<Db>,
    channels: Option<EventChannels>,
    uuids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    for uuid in uuids {
        log::debug!("LOOKING FOR UUID {uuid}");

        let (activity, target_note, target_remote_note, target_profile, target_remote_actor) =
            get_activity_by_uuid(conn, uuid.clone())
                .await
                .ok_or(TaskError::TaskFailed)?;

        log::debug!("FOUND ACTIVITY\n{activity:#?}");
        let profile_id = activity.profile_id.ok_or(TaskError::TaskFailed)?;

        let sender = get_profile(conn, profile_id)
            .await
            .ok_or(TaskError::TaskFailed)?;

        let activity = ApActivity::try_from((
            (
                activity,
                target_note,
                target_remote_note,
                target_profile,
                target_remote_actor,
            ),
            None,
        ))
        .map_err(|_| TaskError::TaskFailed)?;
        let inboxes: Vec<ApAddress> = get_inboxes(activity.clone(), sender.clone()).await;

        send_to_inboxes(inboxes, sender, activity.clone()).await;
    }
    Ok(())
}

pub fn process_follow(job: Job) -> io::Result<()> {
    log::debug!("PROCESSING OUTGOING FOLLOW REQUEST");

    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    handle
        .block_on(async {
            process_follow_task(
                None,
                None,
                serde_json::from_value(job.args().into()).unwrap(),
            )
            .await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub async fn process_accept_task(
    conn: Option<Db>,
    channels: Option<EventChannels>,
    uuids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    for uuid in uuids {
        log::debug!("UUID: {uuid}");

        let extended_accept = get_activity_by_uuid(conn, uuid)
            .await
            .ok_or(TaskError::TaskFailed)?;
        let follow_id = extended_accept
            .0
            .target_activity_id
            .ok_or(TaskError::TaskFailed)?;
        let extended_follow = get_activity(conn, follow_id)
            .await
            .ok_or(TaskError::TaskFailed)?;
        if let Ok(ApActivity::Accept(accept)) = (extended_accept, Some(extended_follow)).try_into()
        {
            if let MaybeReference::Actual(ApActivity::Follow(follow)) = accept.object.clone() {
                let profile = get_profile_by_ap_id(conn, follow.actor.to_string())
                    .await
                    .ok_or(TaskError::TaskFailed)?;
                let mut leader =
                    NewLeader::try_from(*accept.clone()).map_err(|_| TaskError::TaskFailed)?;
                leader.link(profile);

                let leader = create_leader(conn, leader)
                    .await
                    .ok_or(TaskError::TaskFailed)?;
                log::debug!("LEADER CREATED: {}", leader.uuid);
            }
        }
    }

    Ok(())
}

pub fn process_accept(job: Job) -> io::Result<()> {
    log::debug!("PROCESSING INCOMING ACCEPT REQUEST");

    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    handle
        .block_on(async {
            process_accept_task(
                None,
                None,
                serde_json::from_value(job.args().into()).unwrap(),
            )
            .await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
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

pub async fn process_remote_undo_follow_task(
    conn: Option<Db>,
    channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    for ap_id in ap_ids {
        log::debug!("APID: {ap_id}");

        if delete_follower_by_ap_id(conn, ap_id.clone()).await {
            log::info!("FOLLOWER RECORD DELETED: {ap_id}");
        }
    }

    Ok(())
}

pub fn process_remote_undo_follow(job: Job) -> io::Result<()> {
    log::debug!("PROCESSING INCOMING UNDO FOLLOW REQUEST");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    handle
        .block_on(async {
            process_remote_undo_follow_task(
                None,
                None,
                serde_json::from_value(job.args().into()).unwrap(),
            )
            .await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub fn acknowledge_followers(job: Job) -> io::Result<()> {
    log::debug!("running acknowledge job");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    handle
        .block_on(async {
            acknowledge_followers_task(
                None,
                None,
                serde_json::from_value(job.args().into()).unwrap(),
            )
            .await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub async fn acknowledge_followers_task(
    conn: Option<Db>,
    channels: Option<EventChannels>,
    uuids: Vec<String>,
) -> Result<(), TaskError> {
    log::debug!("PROCESSING INCOMING FOLLOW REQUEST");

    let conn = conn.as_ref();

    for uuid in uuids {
        log::debug!("UUID: {uuid}");

        let extended_follow = get_activity_by_uuid(conn, uuid)
            .await
            .ok_or(TaskError::TaskFailed)?;

        let follow = ApFollow::try_from(extended_follow).map_err(|_| TaskError::TaskFailed)?;
        let accept = ApAccept::try_from(follow.clone()).map_err(|_| TaskError::TaskFailed)?;

        let profile = get_profile_by_ap_id(conn, accept.actor.clone().to_string())
            .await
            .ok_or(TaskError::TaskFailed)?;

        let actor = get_actor(profile.clone(), follow.actor.clone().to_string())
            .await
            .ok_or(TaskError::TaskFailed)?
            .0;

        send_activity(
            ApActivity::Accept(Box::new(accept)),
            profile.clone(),
            actor.inbox.clone(),
        )
        .await
        .map_err(|_| TaskError::TaskFailed)?;

        log::info!("ACCEPT SENT: {:#?}", actor.inbox);

        let mut follower = NewFollower::try_from(follow).map_err(|_| TaskError::TaskFailed)?;
        follower.link(profile.clone());

        log::debug!("NEW FOLLOWER\n{follower:#?}");
        if create_follower(conn, follower).await.is_some() {
            log::info!("FOLLOWER CREATED");
        }
    }

    Ok(())
}
