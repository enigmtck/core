use faktory::Job;
use std::io;
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{ApActivity, ApAddress},
    helper::{get_local_identifier, LocalIdentifierType},
    models::{
        activities::{get_activity, get_activity_by_uuid, revoke_activity_by_uuid},
        leaders::delete_leader_by_ap_id_and_profile_id,
        profiles::get_profile,
    },
    runner::{get_inboxes, send_to_inboxes},
    MaybeReference,
};

pub fn process_outbound_undo(job: Job) -> io::Result<()> {
    log::debug!("PROCESSING OUTGOING UNDO REQUEST");
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
            if let Some(profile_id) = activity.profile_id {
                if let (Some(sender), Some(id)) = (
                    handle.block_on(async { get_profile(None, profile_id).await }),
                    activity.target_activity_id,
                ) {
                    let target_activity = handle.block_on(async { get_activity(None, id).await });

                    if let Ok(ap_activity) = ApActivity::try_from((
                        (
                            activity.clone(),
                            target_note,
                            target_remote_note,
                            target_profile,
                            target_remote_actor,
                        ),
                        target_activity.clone(),
                    )) {
                        let inboxes: Vec<ApAddress> = handle.block_on(async {
                            get_inboxes(ap_activity.clone(), sender.clone()).await
                        });
                        log::debug!("INBOXES\n{inboxes:#?}");
                        log::debug!("ACTIVITY\n{activity:#?}");
                        handle.block_on(async {
                            send_to_inboxes(inboxes, sender, ap_activity.clone()).await
                        });

                        if let Some(target_activity) = target_activity {
                            if let Ok(target_activity) =
                                ApActivity::try_from((target_activity, None))
                            {
                                match target_activity {
                                    ApActivity::Follow(follow) => {
                                        if let Some(id) = follow.id {
                                            log::debug!("FOLLOW ID: {id}");
                                            if let Some(identifier) = get_local_identifier(id) {
                                                log::debug!("FOLLOW IDENTIFIER: {identifier:#?}");
                                                if let Some(profile_id) = activity.profile_id {
                                                    if let MaybeReference::Reference(ap_id) =
                                                        follow.object
                                                    {
                                                        if handle.block_on(async {
                                                            delete_leader_by_ap_id_and_profile_id(
                                                                None, ap_id, profile_id,
                                                            )
                                                            .await
                                                        }) && handle.block_on(async {
                                                            revoke_activity_by_uuid(
                                                                None,
                                                                identifier.identifier,
                                                            )
                                                            .await
                                                            .is_ok()
                                                        }) {
                                                            log::info!("LEADER DELETED");
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    ApActivity::Like(like) => {
                                        if let Some(id) = like.id {
                                            log::debug!("LIKE ID: {id}");
                                            if let Some(identifier) = get_local_identifier(id) {
                                                log::debug!("LIKE IDENTIFIER: {identifier:#?}");
                                                if identifier.kind == LocalIdentifierType::Activity
                                                    && handle
                                                        .block_on(async {
                                                            revoke_activity_by_uuid(
                                                                None,
                                                                identifier.identifier,
                                                            )
                                                            .await
                                                        })
                                                        .is_ok()
                                                {
                                                    log::info!("LIKE ACTIVITY REVOKED");
                                                }
                                            }
                                        }
                                    }
                                    ApActivity::Announce(announce) => {
                                        if let Some(id) = announce.id {
                                            log::debug!("ANNOUNCE ID: {id}");
                                            if let Some(identifier) = get_local_identifier(id) {
                                                log::debug!("ANNOUNCE IDENTIFIER: {identifier:#?}");
                                                if identifier.kind == LocalIdentifierType::Activity
                                                    && handle
                                                        .block_on(async {
                                                            revoke_activity_by_uuid(
                                                                None,
                                                                identifier.identifier,
                                                            )
                                                            .await
                                                        })
                                                        .is_ok()
                                                {
                                                    log::info!("ANNOUNCE ACTIVITY REVOKED");
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
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
