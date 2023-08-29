use faktory::Job;
use std::io;

use crate::{
    activity_pub::{ApActivity, ApAddress},
    runner::{
        activity::{get_activity_by_uuid, revoke_activity_by_apid},
        get_inboxes, send_to_inboxes,
        user::get_profile,
    },
};

pub fn process_remote_undo_like(job: Job) -> io::Result<()> {
    log::debug!("running process_remote_undo_like job");

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();
        log::debug!("looking for ap_id: {}", ap_id);

        if revoke_activity_by_apid(&ap_id).is_some() {
            log::debug!("LIKE REVOKED: {ap_id}");
        }
    }

    Ok(())
}

pub fn send_like(job: Job) -> io::Result<()> {
    log::debug!("SENDING LIKE");

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
            if let Some(profile_id) = activity.profile_id {
                if let Some(sender) = get_profile(profile_id) {
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
                        let inboxes: Vec<ApAddress> = get_inboxes(activity.clone(), sender.clone());
                        send_to_inboxes(inboxes, sender, activity.clone());
                    }
                }
            }
        }
    }

    Ok(())
}
