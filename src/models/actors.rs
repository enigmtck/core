use std::collections::HashSet;

use crate::activity_pub::ApActor;
use crate::activity_pub::ApAddress;
use crate::db::Db;
use crate::models::to_serde;
use crate::schema::actors;
use anyhow::anyhow;
use anyhow::Result;
use chrono::DateTime;
use chrono::Utc;
use diesel::prelude::*;
use serde_json::json;

use super::followers::get_followers_by_actor_id;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::actors::NewActor;
        pub use crate::models::pg::actors::Actor;
        pub use crate::models::pg::actors::ActorType;
        pub use crate::models::pg::actors::create_or_update_actor;
        pub use crate::models::pg::actors::get_actor_by_key_id;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::remote_actors::NewRemoteActor;
        pub use crate::models::sqlite::remote_actors::RemoteActor;
        pub use crate::models::sqlite::remote_actors::create_or_update_remote_actor;
    }
}

impl TryFrom<ApActor> for NewActor {
    type Error = anyhow::Error;

    fn try_from(actor: ApActor) -> Result<NewActor, Self::Error> {
        let ek_hashtags = to_serde(&Some(actor.get_hashtags())).unwrap_or_else(|| json!([]));
        let ek_webfinger = actor.get_webfinger();
        let as_id = actor.id.clone().ok_or(anyhow!("no id"))?.to_string();
        let as_type = actor.kind.to_string().try_into()?;
        let as_context = to_serde(&actor.context.clone());
        let as_name = actor.clone().name;
        let as_preferred_username = Some(actor.preferred_username);
        let as_summary = actor.summary;
        let as_inbox = actor.inbox;
        let as_outbox = actor.outbox;
        let as_followers = actor.followers;
        let as_following = actor.following;
        let as_liked = actor.liked;
        let as_public_key = to_serde(&Some(actor.public_key)).unwrap();
        let as_featured = actor.featured;
        let as_featured_tags = actor.featured_tags;
        let as_url = to_serde(&actor.url.clone());
        let ap_manually_approves_followers = actor.manually_approves_followers.unwrap_or_default();
        let as_published = actor.published.and_then(|x| {
            x.parse::<DateTime<chrono::FixedOffset>>()
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        });
        let as_tag = to_serde(&actor.tag).unwrap_or(json!([]));
        let as_attachment = to_serde(&actor.attachment).unwrap_or(json!({}));
        let as_endpoints = to_serde(&actor.endpoints).unwrap_or(json!({}));
        let as_icon = to_serde(&actor.icon).unwrap_or_else(|| json!([]));
        let as_image = to_serde(&actor.image).unwrap_or_else(|| json!({}));
        let as_also_known_as = to_serde(&actor.also_known_as).unwrap_or_else(|| json!([]));
        let as_discoverable = actor.discoverable.unwrap_or_default();
        let ap_capabilities = to_serde(&actor.capabilities).unwrap_or_else(|| json!({}));

        Ok(NewActor {
            as_context,
            as_type,
            as_id,
            ek_webfinger,
            as_name,
            as_preferred_username,
            as_summary,
            as_inbox,
            as_outbox,
            as_followers,
            as_following,
            as_liked,
            as_public_key,
            as_featured,
            as_featured_tags,
            as_url,
            ap_manually_approves_followers,
            as_published,
            as_tag,
            as_attachment,
            as_endpoints,
            as_icon,
            as_image,
            as_also_known_as,
            as_discoverable,
            ap_capabilities,
            ek_hashtags,
            ..Default::default()
        })
    }
}

pub async fn tombstone_actor_by_as_id(conn: &Db, as_id: String) -> Result<Actor> {
    conn.run(move |c| {
        diesel::update(actors::table.filter(actors::as_id.eq(as_id)))
            .set(actors::as_type.eq(ActorType::Tombstone))
            .get_result(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn delete_actor_by_as_id(conn: &Db, as_id: String) -> bool {
    // This function checks if ek_username is null to avoid deleting local user records
    conn.run(move |c| {
        diesel::delete(
            actors::table.filter(actors::as_id.eq(as_id).and(actors::ek_username.is_null())),
        )
        .execute(c)
    })
    .await
    .is_ok()
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

pub async fn get_actor_by_webfinger(conn: &Db, webfinger: String) -> Option<Actor> {
    conn.run(move |c| {
        actors::table
            .filter(actors::ek_webfinger.eq(webfinger))
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

pub async fn get_actor_by_as_id(conn: &Db, as_id: String) -> Result<Actor> {
    conn.run(move |c| {
        actors::table
            .filter(actors::as_id.eq(as_id))
            .first::<Actor>(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn get_follower_inboxes(conn: &Db, actor: Actor) -> Vec<ApAddress> {
    let mut inboxes: HashSet<ApAddress> = HashSet::new();

    for (_follower, actor) in get_followers_by_actor_id(conn, actor.id, None).await {
        inboxes.insert(ApAddress::Address(actor.as_inbox));
    }

    Vec::from_iter(inboxes)
}

pub async fn guaranteed_actor(conn: &Db, profile: Option<Actor>) -> Actor {
    match profile {
        Some(profile) => profile,
        None => get_actor_by_username(conn, (*crate::SYSTEM_USER).clone())
            .await
            .expect("Unable to retrieve system user"),
    }
}
