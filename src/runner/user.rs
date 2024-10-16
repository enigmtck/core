use std::collections::HashSet;

use crate::{
    activity_pub::{ApActivity, ApActor, ApAddress, ApUpdate},
    db::Db,
    fairings::events::EventChannels,
    models::{
        actors::{get_actor_by_uuid, Actor},
        followers::get_followers_by_actor_id,
        remote_actors::get_remote_actor_by_ap_id,
    },
    runner::send_to_inboxes,
};

use super::TaskError;

pub async fn get_follower_inboxes(conn: &Db, profile: Actor) -> Vec<ApAddress> {
    let mut inboxes: HashSet<ApAddress> = HashSet::new();

    for (follower, _) in get_followers_by_actor_id(conn, profile.id).await {
        if let Ok(actor) = get_remote_actor_by_ap_id(None, follower.actor).await {
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
    conn: Option<Db>,
    _channels: Option<EventChannels>,
    uuids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    for uuid in uuids {
        log::debug!("LOOKING UP {uuid}");
        let profile = get_actor_by_uuid(conn.unwrap(), uuid)
            .await
            .ok_or(TaskError::TaskFailed)?;
        log::debug!("FOUND PROFILE");
        let update = ApUpdate::try_from(ApActor::from(profile.clone()))
            .map_err(|_| TaskError::TaskFailed)?;
        log::debug!("UPDATE\n{update:#?}");

        send_to_inboxes(
            get_follower_inboxes(conn.unwrap(), profile.clone()).await,
            profile,
            ApActivity::Update(update),
        )
        .await
        .map_err(|_| TaskError::TaskFailed)?;
    }

    Ok(())
}
