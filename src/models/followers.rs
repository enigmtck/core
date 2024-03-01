use crate::activity_pub::ApFollow;
use crate::db::Db;
use crate::helper::{get_local_identifier, LocalIdentifierType};
use crate::schema::{followers, remote_actors};
use crate::{MaybeReference, POOL};
use diesel::prelude::*;

use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::profiles::Profile;
use super::remote_actors::RemoteActor;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[diesel(table_name = followers)]
pub struct NewFollower {
    pub profile_id: i32,
    pub ap_id: String,
    pub actor: String,
    pub followed_ap_id: String,
    pub uuid: String,
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
    pub fn link(&mut self, profile: Profile) -> &mut Self {
        if let Some(id) = get_local_identifier(self.followed_ap_id.clone()) {
            if id.kind == LocalIdentifierType::User
                && id.identifier.to_lowercase() == profile.username.to_lowercase()
            {
                self.profile_id = profile.id;
                self
            } else {
                self
            }
        } else {
            self
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = followers)]
pub struct Follower {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile_id: i32,
    pub ap_id: String,
    pub actor: String,
    pub followed_ap_id: String,
    pub uuid: String,
}

pub async fn create_follower(conn: Option<&Db>, follower: NewFollower) -> Option<Follower> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(followers::table)
                    .values(&follower)
                    .get_result::<Follower>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(followers::table)
                .values(&follower)
                .get_result::<Follower>(&mut pool)
                .ok()
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

pub async fn get_followers_by_profile_id(
    conn: Option<&Db>,
    profile_id: i32,
) -> Vec<(Follower, Option<RemoteActor>)> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                followers::table
                    .filter(followers::profile_id.eq(profile_id))
                    .left_join(remote_actors::table.on(followers::actor.eq(remote_actors::ap_id)))
                    .order_by(followers::created_at.desc())
                    .get_results::<(Follower, Option<RemoteActor>)>(c)
            })
            .await
            .unwrap_or(vec![]),
        None => POOL.get().map_or(vec![], |mut pool| {
            followers::table
                .filter(followers::profile_id.eq(profile_id))
                .left_join(remote_actors::table.on(followers::actor.eq(remote_actors::ap_id)))
                .order_by(followers::created_at.desc())
                .get_results::<(Follower, Option<RemoteActor>)>(&mut pool)
                .unwrap_or(vec![])
        }),
    }
}
