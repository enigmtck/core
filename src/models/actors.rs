use std::collections::HashSet;

use crate::activity_pub::ApActor;
use crate::activity_pub::ApAddress;
use crate::activity_pub::ApContext;
use crate::db::Db;
use crate::models::to_serde;
use crate::schema::{actors, leaders, profiles, remote_actors};
use crate::POOL;
use anyhow::anyhow;
use anyhow::Result;
use chrono::DateTime;
use chrono::Utc;
use diesel::prelude::*;
use serde_json::json;
use serde_json::Value;

use super::followers::get_followers_by_actor_id;
use super::leaders::Leader;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::actors::NewActor;
        pub use crate::models::pg::actors::Actor;
        pub use crate::models::pg::actors::ActorType;
        pub use crate::models::pg::actors::create_or_update_actor;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::remote_actors::NewRemoteActor;
        pub use crate::models::sqlite::remote_actors::RemoteActor;
        pub use crate::models::sqlite::remote_actors::create_or_update_remote_actor;
    }
}

impl TryFrom<ApActor> for NewActor {
    type Error = anyhow::Error;

    fn try_from(actor: ApActor) -> Result<NewActor, Self::Error> {
        let ap_id = actor.id.clone().ok_or(anyhow!("no id"))?;
        Ok(NewActor {
            as_context: {
                if let Some(context) = actor.context.clone() {
                    to_serde(context)
                } else {
                    to_serde(ApContext::default())
                }
            },
            as_type: actor.kind.to_string().try_into()?,
            as_id: ap_id.to_string(),
            ek_webfinger: actor.clone().get_webfinger(),
            as_name: actor.clone().name,
            as_preferred_username: Some(actor.preferred_username),
            as_summary: actor.summary,
            as_inbox: actor.inbox,
            as_outbox: actor.outbox,
            as_followers: actor.followers,
            as_following: actor.following,
            as_liked: actor.liked,
            as_public_key: to_serde(actor.public_key).unwrap(),
            as_featured: actor.featured,
            as_featured_tags: actor.featured_tags,
            as_url: actor.url,
            ap_manually_approves_followers: actor.manually_approves_followers.unwrap_or_default(),
            as_published: actor
                .published
                .ok_or(anyhow!("no published"))?
                .parse::<DateTime<chrono::FixedOffset>>()
                .ok()
                .map(|dt| dt.with_timezone(&Utc)),
            as_tag: to_serde(actor.tag).unwrap_or(json!([])),
            as_attachment: to_serde(actor.attachment).unwrap_or(json!({})),
            as_endpoints: to_serde(actor.endpoints).unwrap_or(json!({})),
            as_icon: to_serde(actor.icon).unwrap_or(json!({})),
            as_image: to_serde(actor.image).unwrap_or(json!({})),
            as_also_known_as: to_serde(actor.also_known_as).unwrap_or(json!([])),
            as_discoverable: actor.discoverable.unwrap_or_default(),
            ap_capabilities: to_serde(actor.capabilities).unwrap_or(json!({})),
            ..Default::default()
        })
    }
}

pub async fn get_actor(conn: &Db, id: i32) -> Option<Actor> {
    conn.run(move |c| actors::table.find(id).first::<Actor>(c))
        .await
        .ok()
}

pub async fn get_actor_by_username(conn: &Db, username: String) -> Option<Actor> {
    conn.run(move |c| {
        actors::table
            .filter(actors::ek_username.eq(username))
            .first::<Actor>(c)
    })
    .await
    .ok()
}

pub async fn get_actor_by_uuid(conn: &Db, uuid: String) -> Option<Actor> {
    conn.run(move |c| {
        actors::table
            .filter(actors::ek_uuid.eq(uuid))
            .first::<Actor>(c)
    })
    .await
    .ok()
}

pub async fn get_actor_by_as_id(conn: &Db, as_id: String) -> Option<Actor> {
    conn.run(move |c| {
        actors::table
            .filter(actors::as_id.eq(as_id))
            .first::<Actor>(c)
    })
    .await
    .ok()
}

pub async fn get_follower_inboxes(conn: &Db, actor: Actor) -> Vec<ApAddress> {
    let mut inboxes: HashSet<ApAddress> = HashSet::new();

    for (_follower, actor) in get_followers_by_actor_id(conn, actor.id).await {
        if let Some(actor) = actor {
            inboxes.insert(ApAddress::Address(actor.as_inbox));
        }
    }

    Vec::from_iter(inboxes)
}

pub async fn guaranteed_actor(conn: &Db, profile: Option<Actor>) -> Actor {
    match profile {
        Some(profile) => profile,
        None => get_actor_by_username(conn, (*crate::SYSTEM_USER).clone())
            .await
            .expect("unable to retrieve system user"),
    }
}
