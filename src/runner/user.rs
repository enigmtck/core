use std::collections::HashSet;

use faktory::Job;
use std::io;
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{ApActivity, ApActor, ApAddress, ApUpdate},
    admin::{create_user, NewUser},
    models::{
        followers::get_followers_by_profile_id,
        profiles::{get_profile_by_uuid, Profile},
        remote_actors::get_remote_actor_by_ap_id,
    },
    runner::send_to_inboxes,
};

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

pub fn send_profile_update(job: Job) -> io::Result<()> {
    log::debug!("RUNNING SEND_PROFILE_UPDATE JOB");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    let uuids = job.args();

    log::debug!("UUIDS\n{uuids:#?}");

    for uuid in uuids {
        log::debug!("LOOKING UP {uuid}");
        if let Some(profile) = handle
            .block_on(async { get_profile_by_uuid(None, uuid.as_str().unwrap().to_string()).await })
        {
            log::debug!("FOUND PROFILE");
            if let Ok(update) = ApUpdate::try_from(ApActor::from(profile.clone())) {
                log::debug!("UPDATE\n{update:#?}");
                handle.block_on(async {
                    send_to_inboxes(
                        get_follower_inboxes(profile.clone()).await,
                        profile,
                        ApActivity::Update(update),
                    )
                    .await
                });
            }
        }
    }

    Ok(())
}

pub async fn create(user: NewUser) -> Option<Profile> {
    create_user(None, user).await
}
