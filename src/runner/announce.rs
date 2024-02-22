use diesel::prelude::*;
use faktory::Job;
use std::io;
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{ApActivity, ApAddress},
    models::{activities::Activity, remote_notes::RemoteNote},
    runner::{
        activity::{get_activity_by_uuid, revoke_activity_by_apid},
        get_inboxes,
        note::{fetch_remote_note, handle_remote_note},
        send_to_inboxes,
        timeline::get_timeline_item_by_ap_id,
        user::get_profile,
    },
    schema::activities,
    POOL,
};

pub fn send_announce(job: Job) -> io::Result<()> {
    log::debug!("SENDING ANNOUNCE");

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

pub fn process_remote_undo_announce(job: Job) -> io::Result<()> {
    log::debug!("running process_remote_undo_announce job");

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();
        log::debug!("looking for ap_id: {}", ap_id);

        if revoke_activity_by_apid(&ap_id).is_some() {
            log::debug!("ANNOUNCE REVOKED: {ap_id}");
        }
    }

    Ok(())
}

pub fn process_remote_announce(job: Job) -> io::Result<()> {
    log::debug!("running process_remote_announce job");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for uuid in job.args() {
        if let Some(uuid) = uuid.as_str() {
            log::debug!("RETRIEVING ANNOUNCE: {uuid}");

            if let Some((activity, _, _, _, _)) = get_activity_by_uuid(uuid.to_string()) {
                if let Some(target_ap_id) = activity.clone().target_ap_id {
                    if get_timeline_item_by_ap_id(target_ap_id.clone()).is_none() {
                        handle.block_on(async {
                            if let Some(remote_note) = fetch_remote_note(target_ap_id.clone()).await
                            {
                                update_target_remote_note(
                                    activity.clone(),
                                    handle_remote_note(remote_note).await,
                                );
                            };
                        });
                    };

                    // TODO: also update the updated_at time on timeline and surface that in the
                    // client to bump it in the view
                    //link_remote_announces_to_timeline(target_ap_id);
                };
            }
        }
    }

    Ok(())
}

pub fn update_target_remote_note(activity: Activity, remote_note: RemoteNote) -> Option<usize> {
    if let Ok(mut conn) = POOL.get() {
        diesel::update(activities::table.find(activity.id))
            .set(activities::target_remote_note_id.eq(remote_note.id))
            .execute(&mut conn)
            .ok()
    } else {
        None
    }
}
