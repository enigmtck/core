use faktory::Job;
use std::io;

use crate::{
    activity_pub::{ApActivity, ApAddress},
    helper::{get_local_identifier, LocalIdentifierType},
    runner::{
        activity::{get_activity, get_activity_by_uuid, revoke_activity_by_uuid},
        follow::delete_leader_by_ap_id_and_profile_id,
        get_inboxes, send_to_inboxes,
        user::get_profile,
    },
    MaybeReference,
};

pub fn process_outbound_undo(job: Job) -> io::Result<()> {
    log::debug!("PROCESSING OUTGOING UNDO REQUEST");

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
            if let Some(profile_id) = activity.profile_id {
                if let (Some(sender), Some(id)) =
                    (get_profile(profile_id), activity.target_activity_id)
                {
                    let target_activity = get_activity(id);

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
                        let inboxes: Vec<ApAddress> =
                            get_inboxes(ap_activity.clone(), sender.clone());
                        log::debug!("INBOXES\n{inboxes:#?}");
                        log::debug!("ACTIVITY\n{activity:#?}");
                        send_to_inboxes(inboxes, sender, ap_activity.clone());

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
                                                        if delete_leader_by_ap_id_and_profile_id(
                                                            ap_id, profile_id,
                                                        )
                                                        .is_ok()
                                                            && revoke_activity_by_uuid(
                                                                identifier.identifier,
                                                            )
                                                            .is_some()
                                                        {
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
                                                    && revoke_activity_by_uuid(
                                                        identifier.identifier,
                                                    )
                                                    .is_some()
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
                                                    && revoke_activity_by_uuid(
                                                        identifier.identifier,
                                                    )
                                                    .is_some()
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
