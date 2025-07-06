use anyhow::{anyhow, Result};
use rocket::tokio::time::{sleep, Duration};

use crate::{
    db::Db,
    fairings::events::EventChannels,
    helper::get_domain_from_url,
    models::{
        activities::{
            get_activity_by_ap_id, revoke_activity_by_apid, update_target_object,
            TryFromExtendedActivity,
        },
        actors::{get_actor, guaranteed_actor, Actor},
        instances::get_instance_by_domain_name,
        objects::{get_object_by_as_id, Object},
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
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    // ... existing code
    for ap_id in ap_ids {
        let (activity, target_activity, target_object, target_actor) =
            get_activity_by_ap_id(&conn, ap_id.clone())
                .await
                .map_err(|e| {
                    log::error!("DB error retrieving activity {ap_id}: {e}");
                    TaskError::TaskFailed
                })?
                .ok_or_else(|| {
                    log::error!("Failed to retrieve Activity: {ap_id}");
                    TaskError::TaskFailed
                })?;

        let profile_id = activity.actor_id.ok_or(TaskError::TaskFailed)?;
        let sender = get_actor(&conn, profile_id).await.or_else(|_| {
            log::error!("Failed to retrieve Actor: {profile_id}");
            Err(TaskError::TaskFailed)
        })?;

        let activity = ApActivity::try_from_extended_activity((
            activity,
            target_activity,
            target_object,
            target_actor,
        ))
        .map_err(|e| {
            log::error!("Failed to build ApActivity: {e}");
            TaskError::TaskFailed
        })?
        .formalize();

        let inboxes: Vec<ApAddress> = get_inboxes(&conn, activity.clone(), sender.clone()).await;

        send_to_inboxes(&conn, inboxes, sender, activity.clone())
            .await
            .map_err(|e| {
                log::error!("Failed to send Announce: {e}");
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
        if revoke_activity_by_apid(&conn, ap_id.clone()).await.is_ok() {
            log::debug!("Announce revoked: {ap_id}");
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
            .map_err(|e| {
                log::error!("DB error retrieving activity {ap_id}: {e}");
                TaskError::TaskFailed
            })?
            .ok_or_else(|| {
                log::error!("Failed to retrieve Activity: {ap_id}");
                TaskError::TaskFailed
            })?;

    let target_ap_id = activity.clone().target_ap_id.ok_or_else(|| {
        log::error!("target_ap_id can not be None");
        TaskError::TaskFailed
    })?;

    if let Ok(object) = get_object_by_as_id(&conn, target_ap_id.clone()).await {
        update_target_object(&conn, activity, object)
            .await
            .map_err(|e| {
                log::error!("Failed to update target Object: {e}");
                TaskError::TaskFailed
            })?;
    } else {
        let domain_name = get_domain_from_url(target_ap_id.clone()).ok_or(TaskError::TaskFailed)?;
        if get_instance_by_domain_name(&conn, domain_name.clone())
            .await
            .map(|option_instance| option_instance.map_or(false, |instance| instance.blocked))
            .unwrap_or(false)
        // If DB query fails or instance not found, default to not blocked
        {
            log::debug!("Instance is explicitly blocked: {domain_name}");
            return Err(TaskError::Prohibited);
        }

        async fn retrieve_object(
            conn: &Db,
            as_id: String,
            profile: Actor,
            retries: u64,
        ) -> Result<Object> {
            if retries == 0 {
                return Err(anyhow!("Maximum retry limit reached"));
            }

            match fetch_remote_object(conn, as_id.clone(), profile.clone()).await {
                Ok(object) => Ok(object),
                Err(e) => {
                    log::warn!("Failed to retrieve remote Object: {} | {e}", as_id.clone());
                    log::warn!("Remaining attempts: {retries}");
                    let backoff = Duration::from_secs(120 / retries);
                    sleep(backoff).await;
                    Box::pin(retrieve_object(conn, as_id, profile, retries - 1)).await
                }
            }
        }

        let remote_object = retrieve_object(&conn, target_ap_id, profile, 10)
            .await
            .map_err(|e| {
                log::error!("All retries failed: {e}");
                TaskError::TaskFailed
            })?;

        let _ = update_target_object(
            &conn,
            activity.clone(),
            handle_object(
                &conn,
                channels.clone(),
                remote_object,
                Some(activity.actor.clone()),
            )
            .await
            .map_err(|e| {
                log::error!("Failed to update target Object: {e}");
                TaskError::TaskFailed
            })?,
        )
        .await;
    }

    Ok(())
}
