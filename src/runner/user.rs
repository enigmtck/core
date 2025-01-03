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
use jdt_activity_pub::{ApActivity, ApActor, ApAddress, ApUpdate};

use super::TaskError;

pub async fn get_follower_inboxes(conn: &Db, profile: Actor) -> Vec<ApAddress> {
    let mut inboxes: HashSet<ApAddress> = HashSet::new();

    for (follower, _) in get_followers_by_actor_id(conn, profile.id, None).await {
        if let Ok(actor) = get_actor_by_as_id(conn, follower.actor).await {
            let actor = ApActor::from(actor);
            if let Some(endpoints) = actor.endpoints {
                inboxes.insert(ApAddress::Address(endpoints.shared_inbox));
            } else {
                inboxes.insert(ApAddress::Address(actor.inbox));
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
        let profile = get_actor_by_uuid(&conn, uuid)
            .await
            .ok_or(TaskError::TaskFailed)?;

        let update = ApUpdate::try_from(ApActor::from(profile.clone())).map_err(|e| {
            log::error!("FAILED TO BUILD ApUpdate: {e:#?}");
            TaskError::TaskFailed
        })?;

        send_to_inboxes(
            &conn,
            get_follower_inboxes(&conn, profile.clone()).await,
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
