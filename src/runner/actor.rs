use chrono::{Duration, Utc};
use diesel::prelude::*;
use reqwest::StatusCode;

use crate::{
    activity_pub::{retriever::maybe_signed_get, ApActor},
    models::{
        leaders::Leader,
        profiles::Profile,
        remote_actors::{NewRemoteActor, RemoteActor},
    },
    runner::{cache::cache_content, follow::get_leader_by_actor_ap_id_and_profile},
    schema::{leaders, profiles, remote_actors},
    POOL,
};

pub fn get_remote_actor_by_ap_id(ap_id: String) -> Option<RemoteActor> {
    if let Ok(mut conn) = POOL.get() {
        match remote_actors::table
            .filter(remote_actors::ap_id.eq(ap_id))
            .first::<RemoteActor>(&mut conn)
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
    if let Ok(mut conn) = POOL.get() {
        match diesel::insert_into(remote_actors::table)
            .values(&actor)
            .on_conflict(remote_actors::ap_id)
            .do_update()
            .set((&actor, remote_actors::checked_at.eq(Utc::now())))
            .get_result::<RemoteActor>(&mut conn)
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

async fn cache_actor(actor: &ApActor) -> &ApActor {
    if let Some(tags) = actor.tag.clone() {
        for tag in tags {
            let _ = cache_content(tag.try_into()).await;
        }
    };

    for image in vec![actor.image.clone(), actor.icon.clone()]
        .into_iter()
        .flatten()
    {
        let _ = cache_content(Ok(image.clone().into())).await;
    }

    actor
}

pub async fn get_actor(
    profile: Option<Profile>,
    id: String,
) -> Option<(RemoteActor, Option<Leader>)> {
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
            profile.and_then(|x| get_leader_by_actor_ap_id_and_profile(id, x.id)),
        ))
    } else {
        log::debug!("PERFORMING REMOTE LOOKUP FOR ACTOR: {id}");

        if let Ok(resp) = maybe_signed_get(profile, id.clone(), false).await {
            match resp.status() {
                StatusCode::ACCEPTED | StatusCode::OK => {
                    if let Ok(actor) = resp.json::<ApActor>().await {
                        if let Ok(new_remote_actor) =
                            NewRemoteActor::try_from(cache_actor(&actor).await.clone())
                        {
                            create_or_update_remote_actor(new_remote_actor)
                                .map(|a| (a, Option::None))
                        } else {
                            log::debug!(
                                "FAILED TO CONVERT AP_ACTOR TO NEW_REMOTE_ACTOR\n{actor:#?}"
                            );
                            None
                        }
                    } else {
                        log::debug!("FAILED TO DECODE REMOTE ACTOR");
                        None
                    }
                }
                StatusCode::GONE => {
                    log::debug!("REMOTE ACTOR HAS BEEN DELETED AT THE SOURCE");
                    None
                }
                _ => {
                    log::debug!(
                        "REMOTE ACTOR (NOT UPDATED) LOOKUP STATUS: {}",
                        resp.status()
                    );
                    None
                }
            }
        } else {
            None
        }
    }
}

pub fn get_leader_by_endpoint(endpoint: String) -> Option<(RemoteActor, Leader)> {
    if let Ok(mut conn) = POOL.get() {
        match remote_actors::table
            .inner_join(leaders::table.on(leaders::leader_ap_id.eq(remote_actors::ap_id)))
            .filter(remote_actors::followers.eq(endpoint))
            .first::<(RemoteActor, Leader)>(&mut conn)
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
    if let Ok(mut conn) = POOL.get() {
        match remote_actors::table
            .inner_join(leaders::table.on(leaders::leader_ap_id.eq(remote_actors::ap_id)))
            .left_join(profiles::table.on(leaders::profile_id.eq(profiles::id)))
            .filter(remote_actors::followers.eq(endpoint))
            .get_results::<(RemoteActor, Leader, Option<Profile>)>(&mut conn)
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
