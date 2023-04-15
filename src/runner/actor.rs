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

pub fn create_remote_actor(actor: NewRemoteActor) -> Option<RemoteActor> {
    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(remote_actors::table)
            .values(&actor)
            .get_result::<RemoteActor>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::debug!("database failure: {:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub async fn get_actor(profile: Profile, id: String) -> Option<(RemoteActor, Option<Leader>)> {
    match get_remote_actor_by_ap_id(id.clone()) {
        Some(remote_actor) => {
            log::debug!("actor retrieved from storage");

            Option::from((
                remote_actor,
                get_leader_by_actor_ap_id_and_profile(id, profile.id),
            ))
        }
        None => {
            log::debug!("performing remote lookup for actor");

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
                        create_remote_actor(NewRemoteActor::from(actor)).map(|a| (a, Option::None))
                    }
                    StatusCode::GONE => {
                        log::debug!("GONE: {:#?}", resp.status());
                        Option::None
                    }
                    _ => {
                        log::debug!("STATUS: {:#?}", resp.status());
                        Option::None
                    }
                },
                Err(e) => {
                    log::debug!("{:#?}", e);
                    Option::None
                }
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
