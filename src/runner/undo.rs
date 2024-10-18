use crate::{
    activity_pub::{ApActivity, ApAddress},
    db::Db,
    fairings::events::EventChannels,
    helper::{get_activity_ap_id_from_uuid, get_local_identifier, LocalIdentifierType},
    models::{
        activities::{get_activity_by_ap_id, revoke_activity_by_uuid},
        actors::get_actor,
        leaders::delete_leader_by_ap_id_and_profile_id,
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

        let (activity, target_activity, target_object, target_actor) = get_activity_by_ap_id(
            conn.ok_or(TaskError::TaskFailed)?,
            get_activity_ap_id_from_uuid(uuid.clone()),
        )
        .await
        .ok_or(TaskError::TaskFailed)?;

        let profile_id = activity.actor_id.ok_or(TaskError::TaskFailed)?;
        let sender = get_actor(conn.unwrap(), profile_id)
            .await
            .ok_or(TaskError::TaskFailed)?;

        let ap_activity = ApActivity::try_from((
            activity.clone(),
            target_activity.clone(),
            target_object.clone(),
            target_actor.clone(),
        ))
        .map_err(|_| TaskError::TaskFailed)?;
        let inboxes: Vec<ApAddress> = get_inboxes(conn, ap_activity.clone(), sender.clone()).await;
        log::debug!("INBOXES\n{inboxes:#?}");
        log::debug!("ACTIVITY\n{activity:#?}");

        send_to_inboxes(inboxes, sender, ap_activity.clone())
            .await
            .map_err(|_| TaskError::TaskFailed)?;

        let target_activity = ApActivity::try_from((
            target_activity.ok_or(TaskError::TaskFailed)?,
            None,
            target_object,
            target_actor,
        ))
        .map_err(|_| TaskError::TaskFailed)?;
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
