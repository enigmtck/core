use crate::{
    activity_pub::{ApActivity, ApAddress},
    db::Db,
    fairings::events::EventChannels,
    helper::{get_local_identifier, LocalIdentifierType},
    models::{
        activities::{get_activity, get_activity_by_uuid, revoke_activity_by_uuid},
        leaders::delete_leader_by_ap_id_and_profile_id,
        profiles::get_profile,
    },
    runner::{get_inboxes, send_to_inboxes},
    MaybeReference,
};

use super::TaskError;

pub async fn process_outbound_undo_task(
    conn: Option<Db>,
    _channels: Option<EventChannels>,
    uuids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    for uuid in uuids {
        log::debug!("LOOKING FOR UUID {uuid}");

        let (
            activity,
            target_note,
            target_remote_note,
            target_profile,
            target_remote_actor,
            target_remote_question,
            target_remote_note_hashtag,
        ) = get_activity_by_uuid(conn, uuid.clone())
            .await
            .ok_or(TaskError::TaskFailed)?;

        let profile_id = activity.profile_id.ok_or(TaskError::TaskFailed)?;
        let sender = get_profile(conn, profile_id)
            .await
            .ok_or(TaskError::TaskFailed)?;
        let id = activity.target_activity_id.ok_or(TaskError::TaskFailed)?;

        let target_activity = get_activity(conn, id).await;

        let ap_activity = ApActivity::try_from((
            (
                activity.clone(),
                target_note,
                target_remote_note,
                target_profile,
                target_remote_actor,
                target_remote_question,
                target_remote_note_hashtag,
            ),
            target_activity.clone(),
        ))
        .map_err(|_| TaskError::TaskFailed)?;
        let inboxes: Vec<ApAddress> = get_inboxes(conn, ap_activity.clone(), sender.clone()).await;
        log::debug!("INBOXES\n{inboxes:#?}");
        log::debug!("ACTIVITY\n{activity:#?}");

        send_to_inboxes(inboxes, sender, ap_activity.clone())
            .await
            .map_err(|_| TaskError::TaskFailed)?;

        let target_activity = target_activity.ok_or(TaskError::TaskFailed)?;
        let target_activity =
            ApActivity::try_from((target_activity, None)).map_err(|_| TaskError::TaskFailed)?;
        match target_activity {
            ApActivity::Follow(follow) => {
                let id = follow.id.ok_or(TaskError::TaskFailed)?;
                log::debug!("FOLLOW ID: {id}");
                let identifier = get_local_identifier(id).ok_or(TaskError::TaskFailed)?;
                log::debug!("FOLLOW IDENTIFIER: {identifier:#?}");
                let profile_id = activity.profile_id.ok_or(TaskError::TaskFailed)?;
                if let MaybeReference::Reference(ap_id) = follow.object {
                    if delete_leader_by_ap_id_and_profile_id(conn, ap_id, profile_id).await
                        && revoke_activity_by_uuid(conn, identifier.identifier)
                            .await
                            .is_ok()
                    {
                        log::info!("LEADER DELETED");
                    }
                }
            }
            ApActivity::Like(like) => {
                let id = like.id.ok_or(TaskError::TaskFailed)?;
                log::debug!("LIKE ID: {id}");
                let identifier = get_local_identifier(id).ok_or(TaskError::TaskFailed)?;
                log::debug!("LIKE IDENTIFIER: {identifier:#?}");
                if identifier.kind == LocalIdentifierType::Activity
                    && revoke_activity_by_uuid(conn, identifier.identifier)
                        .await
                        .is_ok()
                {
                    log::info!("LIKE ACTIVITY REVOKED");
                }
            }

            ApActivity::Announce(announce) => {
                let id = announce.id.ok_or(TaskError::TaskFailed)?;
                log::debug!("ANNOUNCE ID: {id}");
                let identifier = get_local_identifier(id).ok_or(TaskError::TaskFailed)?;
                log::debug!("ANNOUNCE IDENTIFIER: {identifier:#?}");
                if identifier.kind == LocalIdentifierType::Activity
                    && revoke_activity_by_uuid(conn, identifier.identifier)
                        .await
                        .is_ok()
                {
                    log::info!("ANNOUNCE ACTIVITY REVOKED");
                }
            }
            _ => {}
        }
    }

    Ok(())
}
