use faktory::Job;
use std::io::{self, ErrorKind};
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{ApActivity, ApAddress},
    models::{
        activities::{get_activity_by_uuid, revoke_activity_by_apid, update_target_remote_note},
        profiles::guaranteed_profile,
        timeline::get_timeline_item_by_ap_id,
    },
    runner::{
        get_inboxes,
        note::{fetch_remote_note, handle_remote_note},
        send_to_inboxes,
        user::get_profile,
    },
    POOL,
};

pub fn send_announce(job: Job) -> io::Result<()> {
    log::debug!("SENDING ANNOUNCE");
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();
        log::debug!("LOOKING FOR UUID {uuid}");

        let pool = POOL.get().map_err(|_| io::Error::from(ErrorKind::Other))?;

        if let Some((
            activity,
            target_note,
            target_remote_note,
            target_profile,
            target_remote_actor,
        )) = handle.block_on(async { get_activity_by_uuid(pool.into(), uuid.clone()).await })
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

pub fn process_remote_undo_announce(job: Job) -> io::Result<()> {
    log::debug!("running process_remote_undo_announce job");

    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();
        log::debug!("looking for ap_id: {}", ap_id);

        let pool = POOL.get().map_err(|_| io::Error::from(ErrorKind::Other))?;

        handle.block_on(async move {
            if revoke_activity_by_apid(pool.into(), ap_id.clone())
                .await
                .is_ok()
            {
                log::debug!("ANNOUNCE REVOKED: {ap_id}");
            }
        });
    }

    Ok(())
}

pub fn process_remote_announce(job: Job) -> io::Result<()> {
    log::debug!("running process_remote_announce job");

    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    let pool = POOL.get().map_err(|_| io::Error::from(ErrorKind::Other))?;
    let profile = handle.block_on(async { guaranteed_profile(pool.into(), None).await });

    for uuid in job.args() {
        if let Some(uuid) = uuid.as_str() {
            log::debug!("RETRIEVING ANNOUNCE: {uuid}");

            let pool = POOL.get().map_err(|_| io::Error::from(ErrorKind::Other))?;
            let profile = profile.clone();

            if let Some((activity, _, _, _, _)) = handle
                .block_on(async move { get_activity_by_uuid(pool.into(), uuid.to_string()).await })
            {
                if let Some(target_ap_id) = activity.clone().target_ap_id {
                    let target_ap_id_clone = target_ap_id.clone();
                    if handle
                        .block_on(async move {
                            get_timeline_item_by_ap_id(
                                POOL.get()
                                    .expect("failed to get database connection")
                                    .into(),
                                target_ap_id_clone.clone(),
                            )
                            .await
                        })
                        .is_none()
                    {
                        if let Some(remote_note) = handle.block_on(async move {
                            fetch_remote_note(target_ap_id.clone(), profile.clone()).await
                        }) {
                            handle.block_on(async move {
                                update_target_remote_note(
                                    POOL.get()
                                        .expect("failed to get database connection")
                                        .into(),
                                    activity.clone(),
                                    handle_remote_note(remote_note)
                                        .await
                                        .expect("failed to handle remote note"),
                                )
                                .await
                            });
                        };
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
