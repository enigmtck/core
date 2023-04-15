use std::collections::HashSet;

use diesel::prelude::*;

use crate::{
    models::{followers::Follower, profiles::Profile},
    schema::{followers, profiles},
};

use super::{actor::get_remote_actor_by_ap_id, POOL};

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

pub fn get_profile_by_username(username: String) -> Option<Profile> {
    if let Ok(conn) = POOL.get() {
        match profiles::table
            .filter(profiles::username.eq(username))
            .first::<Profile>(&conn)
        {
            Ok(x) => Option::from(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn get_profile(id: i32) -> Option<Profile> {
    if let Ok(conn) = POOL.get() {
        match profiles::table.find(id).first::<Profile>(&conn) {
            Ok(x) => Option::from(x),
            Err(_) => Option::None,
        }
    } else {
        Option::None
    }
}

pub fn get_follower_inboxes(profile: Profile) -> HashSet<String> {
    let mut inboxes: HashSet<String> = HashSet::new();

    for follower in get_followers_by_profile_id(profile.id) {
        if let Some(actor) = get_remote_actor_by_ap_id(follower.actor) {
            inboxes.insert(actor.inbox);
        }
    }

    inboxes
}

pub fn get_followers_by_profile_id(profile_id: i32) -> Vec<Follower> {
    if let Ok(conn) = POOL.get() {
        match followers::table
            .filter(followers::profile_id.eq(profile_id))
            .order_by(followers::created_at.desc())
            .get_results::<Follower>(&conn)
        {
            Ok(x) => x,
            Err(_) => vec![],
        }
    } else {
        vec![]
    }
}
