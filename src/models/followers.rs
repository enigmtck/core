use crate::activity_pub::ApFollow;
use crate::db::Db;
use crate::helper::{get_local_identifier, LocalIdentifierType};
use crate::schema::{actors, followers};
use crate::{MaybeReference, POOL};
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

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[diesel(table_name = followers)]
pub struct NewFollower {
    pub profile_id: i32,
    pub ap_id: String,
    pub actor: String,
    pub followed_ap_id: String,
    pub uuid: String,
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
    pub fn link(&mut self, actor: Actor) -> &mut Self {
        if let Some(id) = get_local_identifier(self.followed_ap_id.clone()) {
            if let Some(username) = actor.ek_username {
                if id.kind == LocalIdentifierType::User
                    && id.identifier.to_lowercase() == username.to_lowercase()
                {
                    //self.profile_id = profile.id;
                    self.actor_id = actor.id;
                    self
                } else {
                    self
                }
            } else {
                self
            }
        } else {
            self
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

pub async fn get_followers_by_actor_id(conn: &Db, actor_id: i32) -> Vec<(Follower, Option<Actor>)> {
    conn.run(move |c| {
        followers::table
            .filter(followers::actor_id.eq(actor_id))
            .left_join(actors::table.on(followers::actor.eq(actors::as_id)))
            .order_by(followers::created_at.desc())
            .get_results::<(Follower, Option<Actor>)>(c)
    })
    .await
    .unwrap_or(vec![])
}
