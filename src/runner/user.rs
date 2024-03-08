use std::collections::HashSet;

use faktory::Job;
use std::io;
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{ApActivity, ApActor, ApAddress, ApUpdate},
    admin::{create_user, NewUser},
    db::Db,
    fairings::events::EventChannels,
    models::{
        followers::get_followers_by_profile_id,
        profiles::{get_profile_by_uuid, Profile},
        remote_actors::get_remote_actor_by_ap_id,
    },
    runner::send_to_inboxes,
};

use super::TaskError;

pub async fn get_follower_inboxes(profile: Profile) -> Vec<ApAddress> {
    let mut inboxes: HashSet<ApAddress> = HashSet::new();

    for (follower, _) in get_followers_by_profile_id(None, profile.id).await {
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
    channels: Option<EventChannels>,
    uuids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    for uuid in uuids {
        log::debug!("LOOKING UP {uuid}");
        let profile = get_profile_by_uuid(conn, uuid)
            .await
            .ok_or(TaskError::TaskFailed)?;
        log::debug!("FOUND PROFILE");
        let update = ApUpdate::try_from(ApActor::from(profile.clone()))
            .map_err(|_| TaskError::TaskFailed)?;
        log::debug!("UPDATE\n{update:#?}");

        send_to_inboxes(
            get_follower_inboxes(profile.clone()).await,
            profile,
            ApActivity::Update(update),
        )
        .await;
    }

    Ok(())
}

pub fn send_profile_update(job: Job) -> io::Result<()> {
    log::debug!("RUNNING SEND_PROFILE_UPDATE JOB");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    handle
        .block_on(async {
            send_profile_update_task(
                None,
                None,
                serde_json::from_value(job.args().into()).unwrap(),
            )
            .await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub async fn create(user: NewUser) -> Option<Profile> {
    create_user(None, user).await
}
