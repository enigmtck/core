use crate::activity_pub::ApContext;
use crate::db::Db;
use crate::schema::{leaders, profiles, remote_actors};
use crate::POOL;
use crate::{activity_pub::ApActor, helper::handle_option};
use anyhow::Result;
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use super::leaders::Leader;
use super::profiles::Profile;

#[derive(Serialize, Deserialize, Insertable, AsChangeset)]
#[diesel(table_name = remote_actors)]
pub struct NewRemoteActor {
    pub context: String,
    pub kind: String,
    pub ap_id: String,
    pub webfinger: Option<String>,
    pub name: String,
    pub preferred_username: String,
    pub summary: String,
    pub inbox: String,
    pub outbox: String,
    pub followers: Option<String>,
    pub following: Option<String>,
    pub liked: Option<String>,
    pub public_key: String,
    pub featured: Option<String>,
    pub featured_tags: Option<String>,
    pub url: Option<String>,
    pub manually_approves_followers: Option<bool>,
    pub published: Option<String>,
    pub tag: Option<String>,
    pub attachment: Option<String>,
    pub endpoints: Option<String>,
    pub icon: Option<String>,
    pub image: Option<String>,
    pub also_known_as: Option<String>,
    pub discoverable: Option<bool>,
    pub capabilities: Option<String>,
}

impl TryFrom<ApActor> for NewRemoteActor {
    type Error = &'static str;

    fn try_from(actor: ApActor) -> Result<NewRemoteActor, Self::Error> {
        if let Some(name) = actor.name.clone() {
            if let Some(ap_id) = actor.id.clone() {
                Ok(NewRemoteActor {
                    context: {
                        if let Some(context) = actor.context.clone() {
                            serde_json::to_string(&context).unwrap()
                        } else {
                            serde_json::to_string(&ApContext::default()).unwrap()
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
                    public_key: serde_json::to_string(&actor.public_key).unwrap(),
                    featured: actor.featured,
                    featured_tags: actor.featured_tags,
                    url: actor.url,
                    manually_approves_followers: actor.manually_approves_followers,
                    published: actor.published,
                    tag: handle_option(serde_json::to_string(&actor.tag).unwrap()),
                    attachment: handle_option(serde_json::to_string(&actor.attachment).unwrap()),
                    endpoints: handle_option(serde_json::to_string(&actor.endpoints).unwrap()),
                    icon: handle_option(serde_json::to_string(&actor.icon).unwrap()),
                    image: handle_option(serde_json::to_string(&actor.image).unwrap()),
                    also_known_as: handle_option(
                        serde_json::to_string(&actor.also_known_as).unwrap(),
                    ),
                    discoverable: actor.discoverable,
                    capabilities: handle_option(
                        serde_json::to_string(&actor.capabilities).unwrap(),
                    ),
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

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Debug)]
#[diesel(table_name = remote_actors)]
pub struct RemoteActor {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub context: String,
    pub kind: String,
    pub ap_id: String,
    pub name: String,
    pub preferred_username: Option<String>,
    pub summary: Option<String>,
    pub inbox: String,
    pub outbox: String,
    pub followers: Option<String>,
    pub following: Option<String>,
    pub liked: Option<String>,
    pub public_key: Option<String>,
    pub featured: Option<String>,
    pub featured_tags: Option<String>,
    pub url: Option<String>,
    pub manually_approves_followers: Option<bool>,
    pub published: Option<String>,
    pub tag: Option<String>,
    pub attachment: Option<String>,
    pub endpoints: Option<String>,
    pub icon: Option<String>,
    pub image: Option<String>,
    pub also_known_as: Option<String>,
    pub discoverable: Option<bool>,
    pub capabilities: Option<String>,
    pub checked_at: NaiveDateTime,
    pub webfinger: Option<String>,
}

impl RemoteActor {
    pub fn is_stale(&self) -> bool {
        Utc::now().naive_local() - self.updated_at > Duration::days(7)
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

pub async fn create_or_update_remote_actor(
    conn: Option<&Db>,
    actor: NewRemoteActor,
) -> Result<RemoteActor, anyhow::Error> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(remote_actors::table)
                    .values(&actor)
                    .on_conflict(remote_actors::ap_id)
                    .do_update()
                    .set((&actor, remote_actors::checked_at.eq(Utc::now().naive_utc())))
                    .execute(c)
                    .map_err(anyhow::Error::msg)?;

                remote_actors::table
                    .filter(remote_actors::ap_id.eq(&actor.ap_id))
                    .first::<RemoteActor>(c)
                    .map_err(anyhow::Error::msg)
            })
            .await
        }
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            diesel::insert_into(remote_actors::table)
                .values(&actor)
                .on_conflict(remote_actors::ap_id)
                .do_update()
                .set((&actor, remote_actors::checked_at.eq(Utc::now().naive_utc())))
                .execute(&mut pool) // Changed to .execute()
                .map_err(anyhow::Error::msg)?;

            remote_actors::table
                .filter(remote_actors::ap_id.eq(&actor.ap_id))
                .first::<RemoteActor>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
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
