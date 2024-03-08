use anyhow::Result;
use faktory::Job;
use std::io::{self, Error, ErrorKind};
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{ApActivity, ApAddress},
    db::Db,
    models::{
        activities::{get_activity_by_uuid, revoke_activity_by_apid, update_target_remote_note},
        profiles::{get_profile, guaranteed_profile},
        timeline::get_timeline_item_by_ap_id,
    },
    runner::{
        get_inboxes,
        note::{fetch_remote_note, handle_remote_note},
        send_to_inboxes,
    },
};

use super::TaskError;

pub async fn send_announce_task(conn: Option<Db>, uuids: Vec<String>) -> Result<(), TaskError> {
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

pub fn send_announce(job: Job) -> io::Result<()> {
    log::debug!("SENDING ANNOUNCE");
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    handle
        .block_on(async {
            send_announce_task(None, serde_json::from_value(job.args().into()).unwrap()).await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub async fn remote_undo_announce_task(
    conn: Option<Db>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    for ap_id in ap_ids {
        log::debug!("looking for ap_id: {}", ap_id);

        if revoke_activity_by_apid(conn, ap_id.clone()).await.is_ok() {
            log::debug!("ANNOUNCE REVOKED: {ap_id}");
        }
    }

    Ok(())
}

pub fn process_remote_undo_announce(job: Job) -> io::Result<()> {
    log::debug!("running process_remote_undo_announce job");

    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    handle
        .block_on(async {
            remote_undo_announce_task(None, serde_json::from_value(job.args().into()).unwrap())
                .await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub async fn remote_announce_task(conn: Option<Db>, uuids: Vec<String>) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    let profile = guaranteed_profile(None, None).await;

    for uuid in uuids {
        log::debug!("RETRIEVING ANNOUNCE: {uuid}");

        let profile = profile.clone();

        let (activity, _, _, _, _) = get_activity_by_uuid(conn, uuid.to_string())
            .await
            .ok_or(TaskError::TaskFailed)?;

        let target_ap_id = activity.clone().target_ap_id.ok_or(TaskError::TaskFailed)?;

        if get_timeline_item_by_ap_id(conn, target_ap_id.clone())
            .await
            .is_none()
        {
            let remote_note = fetch_remote_note(target_ap_id.clone(), profile.clone())
                .await
                .ok_or(TaskError::TaskFailed)?;

            update_target_remote_note(
                conn,
                activity.clone(),
                handle_remote_note(remote_note, Some(activity.actor))
                    .await
                    .map_err(|_| TaskError::TaskFailed)?,
            )
            .await;
        };
    }

    Ok(())
}

pub fn process_remote_announce(job: Job) -> io::Result<()> {
    log::debug!("running process_remote_announce job");

    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    handle
        .block_on(async {
            remote_announce_task(None, serde_json::from_value(job.args().into()).unwrap()).await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}
