use std::collections::HashSet;

use crate::{
    db::Db,
    fairings::events::EventChannels,
    models::{
        actors::{get_actor_by_as_id, get_actor_by_uuid, Actor},
        followers::get_followers_by_actor_id,
    },
    runner::send_to_inboxes,
};
use anyhow::Result;
use jdt_activity_pub::{ApActivity, ApActor, ApAddress, ApUpdate};

use super::TaskError;

pub async fn get_follower_inboxes(conn_opt: Option<&Db>, profile: Actor) -> Vec<ApAddress> {
    let mut inboxes: HashSet<ApAddress> = HashSet::new();

    for (follower, _) in get_followers_by_actor_id(conn_opt, profile.id, None).await {
        match get_actor_by_as_id(conn_opt, follower.actor).await {
            Ok(actor_model) => {
                let ap_actor = ApActor::from(actor_model);
                if let Some(endpoints) = ap_actor.endpoints {
                    inboxes.insert(ApAddress::Address(endpoints.shared_inbox));
                } else {
                    inboxes.insert(ApAddress::Address(ap_actor.inbox));
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to get actor for follower {}: {:?}",
                    follower.ap_id,
                    e
                );
            }
        }
    }

    Vec::from_iter(inboxes)
}

pub async fn send_profile_update_task(
    conn: Db,
    _channels: Option<EventChannels>,
    uuids: Vec<String>,
) -> Result<(), TaskError> {
    for uuid in uuids {
        let profile = get_actor_by_uuid(Some(&conn), uuid.clone())
            .await
            .map_err(|e| {
                log::error!("Failed to get actor by uuid {}: {:?}", uuid, e);
                TaskError::TaskFailed
            })?;

        let update = ApUpdate::try_from(ApActor::from(profile.clone())).map_err(|e| {
            log::error!("FAILED TO BUILD ApUpdate: {e:#?}");
            TaskError::TaskFailed
        })?;

        send_to_inboxes(
            Some(&conn),
            get_follower_inboxes(Some(&conn), profile.clone()).await,
            profile,
            ApActivity::Update(update),
        )
        .await
        .map_err(|e| {
            log::error!("FAILED TO SEND TO INBOXES: {e:#?}");
            TaskError::TaskFailed
        })?;
    }

    Ok(())
}
