use crate::db::Db;
use anyhow::Result;
use reqwest::StatusCode;

use crate::{
    activity_pub::{retriever::signed_get, ApActor},
    models::{
        leaders::{get_leader_by_profile_id_and_ap_id, Leader},
        profiles::Profile,
        remote_actor_hashtags::{
            create_remote_actor_hashtag, delete_remote_actor_hashtags_by_remote_actor_id,
            NewRemoteActorHashtag,
        },
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

pub async fn update_actor_tags(conn: Option<&Db>, actor: RemoteActor) -> Result<()> {
    let deleted = delete_remote_actor_hashtags_by_remote_actor_id(None, actor.id).await?;

    log::debug!("DELETED {deleted} ACTOR TAGS");

    let new_tags: Vec<NewRemoteActorHashtag> = actor.clone().into();

    for tag in new_tags.iter() {
        log::debug!("ADDING HASHTAG: {}", tag.hashtag);
        create_remote_actor_hashtag(conn, tag.clone()).await;
    }

    Ok(())
}

pub async fn get_actor(
    conn: Option<&Db>,
    profile: Profile,
    id: String,
) -> Option<(RemoteActor, Option<Leader>)> {
    // In the Rocket version of this, there's an option to force it not to make the external
    // call to update to avoid affecting response time to the browser. But here, that's not relevant.
    // And in fact, for local outbound Notes we use this call to check that the local user is
    // represented as a "remote_actor" when adding the Note to the local Timeline.  This function
    // updates that remote_actor record (or creates it).
    let remote_actor = get_remote_actor_by_ap_id(None, id.clone()).await.ok()?;

    if !remote_actor.is_stale() {
        Some((
            remote_actor,
            get_leader_by_profile_id_and_ap_id(None, profile.id, id).await,
        ))
    } else {
        log::debug!("PERFORMING REMOTE LOOKUP FOR ACTOR: {id}");

        let resp = signed_get(profile, id.clone(), false).await.ok()?;
        match resp.status() {
            StatusCode::ACCEPTED | StatusCode::OK => {
                let actor = resp.json::<ApActor>().await.ok()?;
                let new_remote_actor =
                    NewRemoteActor::try_from(cache_actor(&actor).await.clone()).ok()?;

                let remote_actor = create_or_update_remote_actor(conn, new_remote_actor)
                    .await
                    .ok()?;

                update_actor_tags(conn, remote_actor.clone()).await.ok()?;

                Some((remote_actor, None))
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
    }
}
