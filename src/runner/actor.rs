use chrono::{Duration, Utc};
use reqwest::StatusCode;

use crate::{
    activity_pub::{retriever::signed_get, ApActor},
    models::{
        leaders::{get_leader_by_profile_id_and_ap_id, Leader},
        profiles::Profile,
        remote_actors::{
            create_or_update_remote_actor, get_remote_actor_by_ap_id, NewRemoteActor, RemoteActor,
        },
    },
    runner::cache::cache_content,
};

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

pub async fn get_actor(profile: Profile, id: String) -> Option<(RemoteActor, Option<Leader>)> {
    // In the Rocket version of this, there's an option to force it not to make the external
    // call to update to avoid affecting response time to the browser. But here, that's not relevant.
    // And in fact, for local outbound Notes we use this call to check that the local user is
    // represented as a "remote_actor" when adding the Note to the local Timeline.  This function
    // updates that remote_actor record (or creates it).
    let remote_actor = {
        if let Ok(remote_actor) = get_remote_actor_by_ap_id(None, id.clone()).await {
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
            get_leader_by_profile_id_and_ap_id(None, profile.id, id).await,
        ))
    } else {
        log::debug!("PERFORMING REMOTE LOOKUP FOR ACTOR: {id}");

        if let Ok(resp) = signed_get(profile, id.clone(), false).await {
            match resp.status() {
                StatusCode::ACCEPTED | StatusCode::OK => {
                    if let Ok(actor) = resp.json::<ApActor>().await {
                        if let Ok(new_remote_actor) =
                            NewRemoteActor::try_from(cache_actor(&actor).await.clone())
                        {
                            create_or_update_remote_actor(None, new_remote_actor)
                                .await
                                .ok()
                                .map(|a| (a, None))
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
