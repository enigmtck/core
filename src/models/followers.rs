use super::OffsetPaging;
use crate::db::Db;
use crate::helper::{get_local_identifier, LocalIdentifierType};
use crate::schema::{actors, followers};
use crate::POOL;
use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{sql_query, Insertable};
use diesel::{AsChangeset, Identifiable, Queryable};
use jdt_activity_pub::ApFollow;
use jdt_activity_pub::MaybeReference;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::actors::Actor;

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

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = followers)]
pub struct Follower {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ap_id: String,
    pub actor: String,
    pub followed_ap_id: String,
    pub uuid: String,
    pub actor_id: i32,
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
        None => POOL.get().is_ok_and(|mut pool| {
            diesel::delete(followers::table)
                .filter(followers::ap_id.eq(ap_id))
                .execute(&mut pool)
                .is_ok()
        }),
    }
}

pub async fn get_followers_by_actor_id(
    conn_opt: Option<&Db>,
    actor_id: i32,
    paging: Option<OffsetPaging>,
) -> Vec<(Follower, Actor)> {
    // inner join is used to exclude Actor records that have been deleted
    let operation = move |c: &mut PgConnection| {
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

        query
            .get_results::<(Follower, Actor)>(c)
            .map_err(anyhow::Error::from)
    };

    match conn_opt {
        Some(conn) => conn.run(operation).await.unwrap_or_else(|e| {
            log::error!("Failed to get followers by actor id (with DB conn): {e:?}");
            vec![]
        }),
        None => {
            tokio::task::spawn_blocking(move || {
                let mut pool_conn = POOL.get().map_err(anyhow::Error::msg)?;
                operation(&mut pool_conn)
            })
            .await
            .unwrap_or_else(|e| {
                // Handles JoinError
                log::error!(
                    "Failed to get followers by actor id (spawn_blocking task failed): {e:?}"
                );
                Ok(vec![]) // Ok because the outer unwrap_or_else expects Result<Vec, _>
            })
            .unwrap_or_else(|e| {
                // Handles error from operation itself
                log::error!("Failed to get followers by actor id (DB operation failed): {e:?}");
                vec![]
            })
        }
    }
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

pub async fn delete_followers_by_domain_pattern(
    conn: Option<&Db>,
    domain_pattern: String,
) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("DELETE FROM followers WHERE actor COLLATE \"C\" LIKE $1")
            .bind::<Text, _>(format!("https://{domain_pattern}/%"))
            .execute(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}

pub async fn delete_followers_by_followed_ap_id(conn: Option<&Db>, ap_id: String) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("DELETE FROM followers WHERE followed_ap_id = $1")
            .bind::<Text, _>(ap_id)
            .execute(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}

pub async fn delete_followers_by_actor(conn: Option<&Db>, actor: String) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("DELETE FROM followers WHERE actor = $1")
            .bind::<Text, _>(actor)
            .execute(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}
