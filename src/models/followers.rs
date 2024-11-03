use super::OffsetPaging;
use crate::activity_pub::ApFollow;
use crate::db::Db;
use crate::helper::{get_local_identifier, LocalIdentifierType};
use crate::schema::{actors, followers};
use crate::{MaybeReference, POOL};
use anyhow::Result;
use diesel::prelude::*;

use diesel::Insertable;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::actors::Actor;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::followers::Follower;
        pub use crate::models::pg::followers::create_follower;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::followers::Follower;
        pub use crate::models::sqlite::followers::create_follower;
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = followers)]
pub struct NewFollower {
    pub ap_id: String,

    // This is the as_id (usually remote) of the account that is initiating the Follow action.
    pub actor: String,

    pub followed_ap_id: String,
    pub uuid: String,

    // This is the actor record associated with the followed_ap_id. It's confusing.
    pub actor_id: i32,
}

impl TryFrom<ApFollow> for NewFollower {
    type Error = &'static str;

    fn try_from(follow: ApFollow) -> Result<Self, Self::Error> {
        let followed = {
            match follow.object {
                MaybeReference::Reference(followed) => Some(followed),
                _ => None,
            }
        };

        if let Some(followed) = followed {
            Ok(NewFollower {
                ap_id: follow.id.unwrap(),
                actor: follow.actor.to_string(),
                followed_ap_id: followed,
                uuid: Uuid::new_v4().to_string(),
                ..Default::default()
            })
        } else {
            Err("COULD NOT BUILD NEW FOLLOWER")
        }
    }
}

impl NewFollower {
    pub fn link(&mut self, actor: Actor) -> Self {
        if let Some(id) = get_local_identifier(self.followed_ap_id.clone()) {
            if let Some(username) = actor.ek_username {
                if id.kind == LocalIdentifierType::User
                    && id.identifier.to_lowercase() == username.to_lowercase()
                {
                    //self.profile_id = profile.id;
                    self.actor_id = actor.id;
                    self.clone()
                } else {
                    self.clone()
                }
            } else {
                self.clone()
            }
        } else {
            self.clone()
        }
    }
}

pub async fn get_follower_by_uuid(conn: &Db, uuid: String) -> Option<Follower> {
    conn.run(move |c| {
        followers::table
            .filter(followers::uuid.eq(uuid))
            .first::<Follower>(c)
    })
    .await
    .ok()
}

pub async fn delete_follower_by_ap_id(conn: Option<&Db>, ap_id: String) -> bool {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::delete(followers::table)
                    .filter(followers::ap_id.eq(ap_id))
                    .execute(c)
            })
            .await
            .is_ok(),
        None => POOL.get().map_or(false, |mut pool| {
            diesel::delete(followers::table)
                .filter(followers::ap_id.eq(ap_id))
                .execute(&mut pool)
                .is_ok()
        }),
    }
}

pub async fn get_followers_by_actor_id(
    conn: &Db,
    actor_id: i32,
    paging: Option<OffsetPaging>,
) -> Vec<(Follower, Actor)> {
    // inner join is used to exclude Actor records that have been deleted
    conn.run(move |c| {
        let mut query = followers::table
            .filter(followers::actor_id.eq(actor_id))
            .inner_join(actors::table.on(followers::actor.eq(actors::as_id)))
            .order_by(followers::created_at.desc())
            .into_boxed();

        if let Some(paging) = paging {
            query = query
                .limit(paging.limit as i64)
                .offset((paging.page * paging.limit) as i64);
        }

        query.get_results::<(Follower, Actor)>(c)
    })
    .await
    .unwrap_or(vec![])
}

pub async fn get_follower_count_by_actor_id(conn: &Db, actor_id: i32) -> Result<i64> {
    // inner join is used to exclude Actor records that have been deleted
    conn.run(move |c| {
        followers::table
            .filter(followers::actor_id.eq(actor_id))
            .inner_join(actors::table.on(followers::actor.eq(actors::as_id)))
            .count()
            .get_result(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}
