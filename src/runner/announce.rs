use anyhow::Result;

use crate::{
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::{
            get_activity_by_ap_id, revoke_activity_by_apid, update_target_object,
            TryFromExtendedActivity,
        },
        actors::{get_actor, guaranteed_actor},
        objects::get_object_by_as_id,
    },
    runner::{
        get_inboxes,
        note::{fetch_remote_object, handle_object},
        send_to_inboxes,
    },
};
use jdt_activity_pub::{ApActivity, ApAddress};

use super::TaskError;

pub async fn send_announce_task(
    conn: Db,
    channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let _channels = channels.as_ref();

    for ap_id in ap_ids {
        let (activity, target_activity, target_object, target_actor) =
            get_activity_by_ap_id(&conn, ap_id.clone())
                .await
                .ok_or_else(|| {
                    log::error!("FAILED TO RETRIEVE ACTIVITY");
                    TaskError::TaskFailed
                })?;

        let profile_id = activity.actor_id.ok_or(TaskError::TaskFailed)?;
        let sender = get_actor(&conn, profile_id).await.ok_or_else(|| {
            log::error!("FAILED TO RETRIEVE ACTOR");
            TaskError::TaskFailed
        })?;

        let activity = ApActivity::try_from_extended_activity((
            activity,
            target_activity,
            target_object,
            target_actor,
        ))
        .map_err(|e| {
            log::error!("FAILED TO BUILD ApActivity: {e:#?}");
            TaskError::TaskFailed
        })?
        .formalize();

        let inboxes: Vec<ApAddress> = get_inboxes(&conn, activity.clone(), sender.clone()).await;

        send_to_inboxes(&conn, inboxes, sender, activity.clone())
            .await
            .map_err(|e| {
                log::error!("FAILED TO SEND ANNOUNCE: {e:#?}");
                TaskError::TaskFailed
            })?;
    }

    Ok(())
}

pub async fn remote_undo_announce_task(
    conn: Db,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    for ap_id in ap_ids {
        if revoke_activity_by_apid(Some(&conn), ap_id.clone())
            .await
            .is_ok()
        {
            log::debug!("ANNOUNCE REVOKED: {ap_id}");
        }
    }

    Ok(())
}

pub async fn remote_announce_task(
    conn: Db,
    channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let profile = guaranteed_actor(&conn, None).await;

    let ap_id = ap_ids.first().unwrap();

    let profile = profile.clone();

    let (activity, _target_activity, _target_object, _target_actor) =
        get_activity_by_ap_id(&conn, ap_id.clone())
            .await
            .ok_or_else(|| {
                log::error!("FAILED TO RETRIEVE ACTIVITY");
                TaskError::TaskFailed
            })?;

    let target_ap_id = activity.clone().target_ap_id.ok_or_else(|| {
        log::error!("target_ap_id CAN NOT BE NONE");
        TaskError::TaskFailed
    })?;

    if let Ok(object) = get_object_by_as_id(Some(&conn), target_ap_id.clone()).await {
        update_target_object(Some(&conn), activity, object)
            .await
            .ok_or_else(|| {
                log::error!("FAILED TO UPDATE TARGET OBJECT");
                TaskError::TaskFailed
            })?;
    } else {
        let remote_object = fetch_remote_object(&conn, target_ap_id.clone(), profile.clone())
            .await
            .ok_or_else(|| {
                log::error!("FAILED TO RETRIEVE REMOTE OBJECT");
                TaskError::TaskFailed
            })?;

        update_target_object(
            Some(&conn),
            activity.clone(),
            handle_object(
                &conn,
                channels.clone(),
                remote_object,
                Some(activity.actor.clone()),
            )
            .await
            .map_err(|e| {
                log::error!("FAILED TO UPDATE TARGET OBJECT: {e:#?}");
                TaskError::TaskFailed
            })?,
        )
        .await;
    }

    Ok(())
}
