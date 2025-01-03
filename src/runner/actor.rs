use crate::{
    db::Db,
    models::{
        actors::{create_or_update_actor, get_actor_by_as_id, Actor, NewActor},
        leaders::get_leader_by_actor_id_and_ap_id,
    },
    GetWebfinger,
};
use crate::{models::leaders::Leader, retriever::signed_get, runner::cache::cache_content};
use jdt_activity_pub::ApActor;
use jdt_maybe_multiple::MaybeMultiple;
use reqwest::StatusCode;
async fn cache_actor(conn: &Db, actor: &ApActor) -> ApActor {
    if let MaybeMultiple::Multiple(tags) = actor.tag.clone() {
        for tag in tags {
            let _ = cache_content(conn, tag.try_into()).await;
        }
    };

    for image in vec![actor.image.clone(), actor.icon.clone()]
        .into_iter()
        .flatten()
    {
        let _ = cache_content(conn, Ok(image.clone().into())).await;
    }

    actor.clone()
}

pub async fn get_actor(
    conn: Option<&Db>,
    profile: Actor,
    id: String,
) -> Option<(Actor, Option<Leader>)> {
    // In the Rocket version of this, there's an option to force it not to make the external
    // call to update to avoid affecting response time to the browser. But here, that's not relevant.
    // And in fact, for local outbound Notes we use this call to check that the local user is
    // represented as a "remote_actor" when adding the Note to the local Timeline.  This function
    // updates that remote_actor record (or creates it).
    let remote_actor = get_actor_by_as_id(conn.unwrap(), id.clone()).await.ok()?;

    if !remote_actor.is_stale() {
        Some((
            remote_actor,
            get_leader_by_actor_id_and_ap_id(conn.unwrap(), profile.id, id).await,
        ))
    } else {
        log::debug!("PERFORMING REMOTE LOOKUP FOR ACTOR: {id}");

        let resp = signed_get(profile, id.clone(), false).await.ok()?;
        match resp.status() {
            StatusCode::ACCEPTED | StatusCode::OK => {
                let actor = resp.json::<ApActor>().await.ok()?;
                let webfinger = actor.get_webfinger().await;

                let mut new_remote_actor =
                    NewActor::try_from(cache_actor(conn.unwrap(), &actor).await.clone()).ok()?;
                new_remote_actor.ek_webfinger = webfinger;

                let remote_actor = create_or_update_actor(conn, new_remote_actor).await.ok()?;

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
