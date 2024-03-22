use crate::activity_pub::ApActor;
use crate::activity_pub::ApContext;
use crate::db::Db;
use crate::models::to_serde;
use crate::schema::{leaders, profiles, remote_actors};
use crate::POOL;
use anyhow::Result;
use diesel::prelude::*;
use diesel::Identifiable;

use super::leaders::Leader;
use super::profiles::Profile;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::remote_actors::NewRemoteActor;
        pub use crate::models::pg::remote_actors::RemoteActor;
        pub use crate::models::pg::remote_actors::create_or_update_remote_actor;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::remote_actors::NewRemoteActor;
        pub use crate::models::sqlite::remote_actors::RemoteActor;
        pub use crate::models::sqlite::remote_actors::create_or_update_remote_actor;
    }
}

impl TryFrom<ApActor> for NewRemoteActor {
    type Error = &'static str;

    fn try_from(actor: ApActor) -> Result<NewRemoteActor, Self::Error> {
        if let Some(name) = actor.name.clone() {
            if let Some(ap_id) = actor.id.clone() {
                Ok(NewRemoteActor {
                    context: {
                        if let Some(context) = actor.context.clone() {
                            to_serde(context).unwrap()
                        } else {
                            to_serde(ApContext::default()).unwrap()
                        }
                    },
                    kind: actor.kind.to_string(),
                    ap_id: ap_id.to_string(),
                    webfinger: actor.clone().get_webfinger(),
                    name,
                    preferred_username: actor.preferred_username,
                    summary: actor.summary.unwrap_or_default(),
                    inbox: actor.inbox,
                    outbox: actor.outbox,
                    followers: actor.followers,
                    following: actor.following,
                    liked: actor.liked,
                    public_key: to_serde(actor.public_key).unwrap(),
                    featured: actor.featured,
                    featured_tags: actor.featured_tags,
                    url: actor.url,
                    manually_approves_followers: actor.manually_approves_followers,
                    published: actor.published,
                    tag: to_serde(actor.tag),
                    attachment: to_serde(actor.attachment),
                    endpoints: to_serde(actor.endpoints),
                    icon: to_serde(actor.icon),
                    image: to_serde(actor.image),
                    also_known_as: to_serde(actor.also_known_as),
                    discoverable: actor.discoverable,
                    capabilities: to_serde(actor.capabilities),
                })
            } else {
                log::error!("FAILED TO CONVERT AP_ACTOR TO NEW_REMOTE_ACTOR (NO ID)\n{actor:#?}");
                Err("ACTOR DOES NOT SPECIFY AN ID")
            }
        } else {
            log::error!("FAILED TO CONVERT AP_ACTOR TO NEW_REMOTE_ACTOR (NO NAME)\n{actor:#?}");
            Err("ACTOR DOES NOT SPECIFY A NAME")
        }
    }
}

pub async fn get_remote_actor_by_url(conn: &Db, url: String) -> Option<RemoteActor> {
    conn.run(move |c| {
        remote_actors::table
            .filter(remote_actors::url.eq(url))
            .first::<RemoteActor>(c)
    })
    .await
    .ok()
}

pub async fn delete_remote_actor_by_ap_id(conn: &Db, remote_actor_ap_id: String) -> bool {
    conn.run(move |c| {
        diesel::delete(remote_actors::table.filter(remote_actors::ap_id.eq(remote_actor_ap_id)))
            .execute(c)
    })
    .await
    .is_ok()
}

pub async fn get_remote_actor_by_ap_id(conn: Option<&Db>, apid: String) -> Result<RemoteActor> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                remote_actors::table
                    .filter(remote_actors::ap_id.eq(apid))
                    .first::<RemoteActor>(c)
                    .map_err(anyhow::Error::msg)
            })
            .await
        }
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            remote_actors::table
                .filter(remote_actors::ap_id.eq(apid))
                .first::<RemoteActor>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}

pub async fn get_remote_actor_by_webfinger(conn: &Db, webfinger: String) -> Option<RemoteActor> {
    conn.run(move |c| {
        remote_actors::table
            .filter(remote_actors::webfinger.eq(webfinger))
            .first::<RemoteActor>(c)
    })
    .await
    .ok()
}

pub async fn get_leader_by_endpoint(
    conn: Option<&Db>,
    endpoint: String,
) -> Option<(RemoteActor, Leader)> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                remote_actors::table
                    .inner_join(leaders::table.on(leaders::leader_ap_id.eq(remote_actors::ap_id)))
                    .filter(remote_actors::followers.eq(endpoint))
                    .first::<(RemoteActor, Leader)>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            remote_actors::table
                .inner_join(leaders::table.on(leaders::leader_ap_id.eq(remote_actors::ap_id)))
                .filter(remote_actors::followers.eq(endpoint))
                .first::<(RemoteActor, Leader)>(&mut pool)
                .ok()
        }
    }
}

pub async fn get_follower_profiles_by_endpoint(
    conn: Option<&Db>,
    endpoint: String,
) -> Vec<(RemoteActor, Leader, Option<Profile>)> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                remote_actors::table
                    .inner_join(leaders::table.on(leaders::leader_ap_id.eq(remote_actors::ap_id)))
                    .left_join(profiles::table.on(leaders::profile_id.eq(profiles::id)))
                    .filter(remote_actors::followers.eq(endpoint))
                    .get_results::<(RemoteActor, Leader, Option<Profile>)>(c)
            })
            .await
            .unwrap_or(vec![]),
        None => POOL.get().map_or(vec![], |mut pool| {
            remote_actors::table
                .inner_join(leaders::table.on(leaders::leader_ap_id.eq(remote_actors::ap_id)))
                .left_join(profiles::table.on(leaders::profile_id.eq(profiles::id)))
                .filter(remote_actors::followers.eq(endpoint))
                .get_results::<(RemoteActor, Leader, Option<Profile>)>(&mut pool)
                .unwrap_or(vec![])
        }),
    }
}
