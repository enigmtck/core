use faktory::Job;
use std::io;
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{ApActivity, ApAddress},
    models::{
        activities::{get_activity_by_uuid, revoke_activity_by_apid},
        profiles::get_profile,
    },
    runner::{get_inboxes, send_to_inboxes},
};

pub fn process_remote_undo_like(job: Job) -> io::Result<()> {
    log::debug!("running process_remote_undo_like job");
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();
        log::debug!("looking for ap_id: {}", ap_id);

        if handle
            .block_on(async { revoke_activity_by_apid(None, ap_id.clone()).await })
            .is_ok()
        {
            log::debug!("LIKE REVOKED: {ap_id}");
        }
    }

    Ok(())
}

pub fn send_like(job: Job) -> io::Result<()> {
    log::debug!("SENDING LIKE");

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
