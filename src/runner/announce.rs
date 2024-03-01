use faktory::Job;
use std::io::{Error, ErrorKind, Result};
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{ApActivity, ApAddress},
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

pub fn send_announce(job: Job) -> Result<()> {
    log::debug!("SENDING ANNOUNCE");
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

pub fn process_remote_undo_announce(job: Job) -> Result<()> {
    log::debug!("running process_remote_undo_announce job");

    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();
        log::debug!("looking for ap_id: {}", ap_id);

        handle.block_on(async move {
            if revoke_activity_by_apid(None, ap_id.clone()).await.is_ok() {
                log::debug!("ANNOUNCE REVOKED: {ap_id}");
            }
        });
    }

    Ok(())
}

pub fn process_remote_announce(job: Job) -> Result<()> {
    log::debug!("running process_remote_announce job");

    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    let profile = handle.block_on(async { guaranteed_profile(None, None).await });

    for uuid in job.args() {
        if let Some(uuid) = uuid.as_str() {
            log::debug!("RETRIEVING ANNOUNCE: {uuid}");

            let profile = profile.clone();

            let (activity, _, _, _, _) = handle
                .block_on(async { get_activity_by_uuid(None, uuid.to_string()).await })
                .ok_or("failed to retrieve activity")
                .map_err(|e| Error::new(ErrorKind::Other, e))?;

            let target_ap_id = activity
                .clone()
                .target_ap_id
                .ok_or("target_ap_id is none")
                .map_err(|e| Error::new(ErrorKind::Other, e))?;

            if handle
                .block_on(async { get_timeline_item_by_ap_id(None, target_ap_id.clone()).await })
                .is_none()
            {
                let remote_note = handle
                    .block_on(async {
                        fetch_remote_note(target_ap_id.clone(), profile.clone()).await
                    })
                    .ok_or("failed to fetch remote note")
                    .map_err(|e| Error::new(ErrorKind::Other, e))?;

                handle.block_on(async {
                    update_target_remote_note(
                        None,
                        activity.clone(),
                        handle_remote_note(remote_note, Some(activity.actor))
                            .await
                            .expect("failed to handle remote note"),
                    )
                    .await
                });
            };

            // TODO: also update the updated_at time on timeline and surface that in the
            // client to bump it in the view
            //link_remote_announces_to_timeline(target_ap_id);
        };
    }

    Ok(())
}
