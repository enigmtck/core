use chrono::{Duration, Utc};
use diesel::prelude::*;
use reqwest::{Client, StatusCode};

use crate::{
    activity_pub::ApActor,
    models::{
        leaders::Leader,
        profiles::Profile,
        remote_actors::{NewRemoteActor, RemoteActor},
    },
    runner::follow::get_leader_by_actor_ap_id_and_profile,
    schema::{leaders, profiles, remote_actors},
    signing::{Method, SignParams},
};

use super::POOL;

pub fn get_remote_actor_by_ap_id(ap_id: String) -> Option<RemoteActor> {
    if let Ok(conn) = POOL.get() {
        match remote_actors::table
            .filter(remote_actors::ap_id.eq(ap_id))
            .first::<RemoteActor>(&conn)
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

pub fn create_or_update_remote_actor(actor: NewRemoteActor) -> Option<RemoteActor> {
    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(remote_actors::table)
            .values(&actor)
            .on_conflict(remote_actors::ap_id)
            .do_update()
            .set((&actor, remote_actors::checked_at.eq(Utc::now())))
            .get_result::<RemoteActor>(&conn)
            .optional()
        {
            Ok(x) => x,
            Err(e) => {
                log::debug!("database failure: {:#?}", e);
                None
            }
        }
    } else {
        Option::None
    }
}

pub async fn get_actor(profile: Profile, id: String) -> Option<(RemoteActor, Option<Leader>)> {
    // In the Rocket version of this, there's an option to force it not to make the external
    // call to update to avoid affecting response time to the browser. But here, that's not relevant.
    // And in fact, for local outbound Notes we use this call to check that the local user is
    // represented as a "remote_actor" when adding the Note to the local Timeline.  This function
    // updates that remote_actor record (or creates it).
    let remote_actor = {
        if let Some(remote_actor) = get_remote_actor_by_ap_id(id.clone()) {
            let now = Utc::now();
            let updated = remote_actor.updated_at;

            if now - updated > Duration::days(7) {
                log::debug!("ACTOR EXISTS BUT IS STALE: {id}");
                None
            } else {
                Some(remote_actor)
            }
        } else {
            None
        }
    };

    if let Some(remote_actor) = remote_actor {
        Some((
            remote_actor,
            get_leader_by_actor_ap_id_and_profile(id, profile.id),
        ))
    } else {
        log::debug!("PERFORMING REMOTE LOOKUP FOR ACTOR");

        let url = id.clone();
        let body = Option::None;
        let method = Method::Get;

        let signature = crate::signing::sign(SignParams {
            profile,
            url,
            body,
            method,
        });

        let client = Client::new();
        match client
            .get(&id)
            .header("Signature", &signature.signature)
            .header("Date", signature.date)
            .header(
                "Accept",
                "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"",
            )
            .send()
            .await
        {
            Ok(resp) => match resp.status() {
                StatusCode::ACCEPTED | StatusCode::OK => {
                    let actor: ApActor = resp.json().await.unwrap();
                    create_or_update_remote_actor(NewRemoteActor::from(actor))
                        .map(|a| (a, Option::None))
                }
                StatusCode::GONE => {
                    log::debug!("REMOTE ACTOR HAS BEEN DELETED AT THE SOURCE");
                    Option::None
                }
                _ => {
                    log::debug!(
                        "REMOTE ACTOR (NOT UPDATED) LOOKUP STATUS: {}",
                        resp.status()
                    );
                    Option::None
                }
            },
            Err(e) => {
                log::debug!("REMOTE ACTOR LOOKUP ERROR: {e:#?}");
                Option::None
            }
        }
    }
}

pub fn get_leader_by_endpoint(endpoint: String) -> Option<(RemoteActor, Leader)> {
    if let Ok(conn) = POOL.get() {
        match remote_actors::table
            .inner_join(leaders::table.on(leaders::leader_ap_id.eq(remote_actors::ap_id)))
            .filter(remote_actors::followers.eq(endpoint))
            .first::<(RemoteActor, Leader)>(&conn)
        {
            Ok(x) => Option::from(x),
            Err(_) => Option::None,
        }
    } else {
        Option::None
    }
}

pub fn get_follower_profiles_by_endpoint(
    endpoint: String,
) -> Vec<(RemoteActor, Leader, Option<Profile>)> {
    if let Ok(conn) = POOL.get() {
        match remote_actors::table
            .inner_join(leaders::table.on(leaders::leader_ap_id.eq(remote_actors::ap_id)))
            .left_join(profiles::table.on(leaders::profile_id.eq(profiles::id)))
            .filter(remote_actors::followers.eq(endpoint))
            .get_results::<(RemoteActor, Leader, Option<Profile>)>(&conn)
        {
            Ok(x) => x,
            Err(_) => {
                vec![]
            }
        }
    } else {
        vec![]
    }
}
