use std::collections::HashSet;

use diesel::prelude::*;
use faktory::Job;
use std::io;

use crate::{
    activity_pub::{ApActivity, ApActor, ApAddress, ApUpdate},
    admin::{create_user, NewUser},
    models::{followers::Follower, profiles::Profile},
    runner::send_to_inboxes,
    schema::{followers, profiles},
    FlexibleDb, POOL,
};

use super::actor::get_remote_actor_by_ap_id;

pub fn get_profile_by_ap_id(ap_id: String) -> Option<Profile> {
    let server_url = (*crate::SERVER_URL).clone();

    let id_re = regex::Regex::new(&format!(r#"{server_url}/user/([a-zA-Z0-9_]+)"#)).unwrap();

    if let Some(captures) = id_re.captures(&ap_id) {
        log::debug!("captures\n{captures:#?}");

        if captures.len() == 2 {
            if let Some(username) = captures.get(1) {
                get_profile_by_username(username.as_str().to_string())
            } else {
                Option::None
            }
        } else {
            Option::None
        }
    } else {
        Option::None
    }
}

pub fn get_profile_by_uuid(uuid: String) -> Option<Profile> {
    if let Ok(mut conn) = POOL.get() {
        profiles::table
            .filter(profiles::uuid.eq(uuid))
            .first::<Profile>(&mut conn)
            .ok()
    } else {
        Option::None
    }
}

pub fn get_profile_by_username(username: String) -> Option<Profile> {
    if let Ok(mut conn) = POOL.get() {
        profiles::table
            .filter(profiles::username.eq(username))
            .first::<Profile>(&mut conn)
            .ok()
    } else {
        Option::None
    }
}

pub fn get_profile(id: i32) -> Option<Profile> {
    if let Ok(mut conn) = POOL.get() {
        profiles::table.find(id).first::<Profile>(&mut conn).ok()
    } else {
        Option::None
    }
}

pub fn get_follower_inboxes(profile: Profile) -> Vec<ApAddress> {
    let mut inboxes: HashSet<ApAddress> = HashSet::new();

    for follower in get_followers_by_profile_id(profile.id) {
        if let Some(actor) = get_remote_actor_by_ap_id(follower.actor) {
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

    let uuids = job.args();

    log::debug!("UUIDS\n{uuids:#?}");

    for uuid in uuids {
        log::debug!("LOOKING UP {uuid}");
        if let Some(profile) = get_profile_by_uuid(uuid.as_str().unwrap().to_string()) {
            log::debug!("FOUND PROFILE");
            if let Ok(update) = ApUpdate::try_from(ApActor::from(profile.clone())) {
                log::debug!("UPDATE\n{update:#?}");
                send_to_inboxes(
                    get_follower_inboxes(profile.clone()),
                    profile,
                    ApActivity::Update(update),
                );
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
