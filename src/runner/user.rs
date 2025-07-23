use std::collections::HashSet;

use crate::{
    db::runner::DbRunner,
    events::EventChannels,
    models::{
        activities::{create_activity, NewActivity},
        actors::{get_actor_by_as_id, get_actor_by_uuid, Actor},
        follows::get_followers_by_actor_id,
        //followers::get_followers_by_actor_id,
    },
    runner::{get_inboxes, send_to_inboxes},
};
use anyhow::Result;
use jdt_activity_pub::{ApActivity, ApActor, ApAddress, ApUpdate};

use super::TaskError;

pub async fn get_follower_inboxes<C: DbRunner>(conn: &C, profile: Actor) -> Vec<ApAddress> {
    let mut inboxes: HashSet<ApAddress> = HashSet::new();

    for (follow, _) in get_followers_by_actor_id(conn, profile.id, None)
        .await
        .unwrap_or_default()
    {
        match get_actor_by_as_id(conn, follow.follower_ap_id.clone()).await {
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
                    follow.follower_ap_id,
                    e
                );
            }
        }
    }

    Vec::from_iter(inboxes)
}

pub async fn send_actor_update_task<C: DbRunner>(
    conn: &C,
    _channels: Option<EventChannels>,
    uuids: Vec<String>,
) -> Result<(), TaskError> {
    for uuid in uuids {
        log::debug!("Processing Actor {uuid}");
        let actor = get_actor_by_uuid(conn, uuid.clone()).await.map_err(|e| {
            log::error!("Failed to get actor by uuid {uuid}: {e:?}");
            TaskError::TaskFailed
        })?;

        let mut update = ApUpdate::try_from(ApActor::from(actor.clone())).map_err(|e| {
            log::error!("Failed to build ApUpdate: {e:#?}");
            TaskError::TaskFailed
        })?;

        let mut new_activity = NewActivity::try_from((
            ApActivity::Update(update.clone()),
            Some(actor.clone().into()),
        ))
        .map_err(|_| TaskError::TaskFailed)?;

        new_activity = new_activity.link_actor(conn).await;

        let activity = create_activity(conn, new_activity)
            .await
            .map_err(|_| TaskError::TaskFailed)?;

        update.id = Some(activity.ap_id.ok_or(TaskError::TaskFailed)?);

        log::debug!("Sending update: {update}");

        send_to_inboxes(
            conn,
            get_inboxes(conn, ApActivity::Update(update.clone()), actor.clone()).await,
            actor,
            ApActivity::Update(update),
        )
        .await
        .map_err(|e| {
            log::error!("Failed to send to inboxes: {e:#?}");
            TaskError::TaskFailed
        })?;
    }

    Ok(())
}
