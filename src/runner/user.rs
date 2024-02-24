use std::collections::HashSet;

use diesel::prelude::*;
use faktory::Job;
use std::io;
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{ApActivity, ApActor, ApAddress, ApUpdate},
    admin::{create_user, NewUser},
    db::FlexibleDb,
    models::{followers::Follower, profiles::Profile, remote_actors::get_remote_actor_by_ap_id},
    runner::send_to_inboxes,
    schema::{followers, profiles},
    POOL,
};

pub fn get_profile_by_ap_id(ap_id: String) -> Option<Profile> {
    let server_url = (*crate::SERVER_URL).clone();

    let id_re = regex::Regex::new(&format!(r#"{server_url}/user/([a-zA-Z0-9_]+)"#)).unwrap();

    if let Some(captures) = id_re.captures(&ap_id) {
        log::debug!("captures\n{captures:#?}");

        if captures.len() == 2 {
            if let Some(username) = captures.get(1) {
                get_profile_by_username(username.as_str().to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

pub fn get_profile_by_uuid(uuid: String) -> Option<Profile> {
    if let Ok(mut conn) = POOL.get() {
        profiles::table
            .filter(profiles::uuid.eq(uuid))
            .first::<Profile>(&mut conn)
            .ok()
    } else {
        None
    }
}

pub fn get_profile_by_username(username: String) -> Option<Profile> {
    if let Ok(mut conn) = POOL.get() {
        profiles::table
            .filter(profiles::username.eq(username))
            .first::<Profile>(&mut conn)
            .ok()
    } else {
        None
    }
}

pub fn get_profile(id: i32) -> Option<Profile> {
    if let Ok(mut conn) = POOL.get() {
        profiles::table.find(id).first::<Profile>(&mut conn).ok()
    } else {
        None
    }
}

pub fn get_follower_inboxes(profile: Profile) -> Vec<ApAddress> {
    let mut inboxes: HashSet<ApAddress> = HashSet::new();

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for follower in get_followers_by_profile_id(profile.id) {
        if let Ok(actor) = handle.block_on(async {
            get_remote_actor_by_ap_id(
                POOL.get()
                    .expect("failed to get database connection")
                    .into(),
                follower.actor,
            )
            .await
        }) {
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

pub fn get_followers_by_profile_id(profile_id: i32) -> Vec<Follower> {
    if let Ok(mut conn) = POOL.get() {
        match followers::table
            .filter(followers::profile_id.eq(profile_id))
            .order_by(followers::created_at.desc())
            .get_results::<Follower>(&mut conn)
        {
            Ok(x) => x,
            Err(_) => vec![],
        }
    } else {
        vec![]
    }
}

pub fn send_profile_update(job: Job) -> io::Result<()> {
    log::debug!("RUNNING SEND_PROFILE_UPDATE JOB");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    let uuids = job.args();

    log::debug!("UUIDS\n{uuids:#?}");

    for uuid in uuids {
        log::debug!("LOOKING UP {uuid}");
        if let Some(profile) = get_profile_by_uuid(uuid.as_str().unwrap().to_string()) {
            log::debug!("FOUND PROFILE");
            if let Ok(update) = ApUpdate::try_from(ApActor::from(profile.clone())) {
                log::debug!("UPDATE\n{update:#?}");
                handle.block_on(async {
                    send_to_inboxes(
                        get_follower_inboxes(profile.clone()),
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
    if let Ok(conn) = POOL.get() {
        create_user(FlexibleDb::Pool(conn), user).await
    } else {
        None
    }
}
