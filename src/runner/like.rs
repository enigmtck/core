use crate::{
    activity_pub::{ApActivity, ApAddress},
    db::Db,
    fairings::events::EventChannels,
    helper::get_activity_ap_id_from_uuid,
    models::{
        activities::{get_activity_by_ap_id, revoke_activity_by_apid},
        actors::get_actor,
    },
    runner::{get_inboxes, send_to_inboxes, TaskError},
};

pub async fn process_remote_undo_like_task(
    conn: Option<Db>,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    for ap_id in ap_ids {
        log::debug!("looking for ap_id: {}", ap_id);

        if revoke_activity_by_apid(conn, ap_id.clone()).await.is_ok() {
            log::debug!("LIKE REVOKED: {ap_id}");
        }
    }

    Ok(())
}

pub async fn send_like_task(
    conn: Option<Db>,
    _channels: Option<EventChannels>,
    uuids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    for uuid in uuids {
        log::debug!("LOOKING FOR UUID {uuid}");

        let (activity, target_activity, target_object) = get_activity_by_ap_id(
            conn.ok_or(TaskError::TaskFailed)?,
            get_activity_ap_id_from_uuid(uuid.clone()),
        )
        .await
        .ok_or(TaskError::TaskFailed)?;

        log::debug!("FOUND ACTIVITY\n{activity:#?}");
        let profile_id = activity.actor_id.ok_or(TaskError::TaskFailed)?;

        let sender = get_actor(conn.unwrap(), profile_id)
            .await
            .ok_or(TaskError::TaskFailed)?;

        let activity = ApActivity::try_from(((activity, target_activity, target_object), None))
            .map_err(|_| TaskError::TaskFailed)?;

        let inboxes: Vec<ApAddress> = get_inboxes(conn, activity.clone(), sender.clone()).await;

        send_to_inboxes(inboxes, sender, activity.clone())
            .await
            .map_err(|_| TaskError::TaskFailed)?;
    }

    Ok(())
}
