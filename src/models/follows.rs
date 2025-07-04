use super::OffsetPaging;
use crate::db::Db;
use crate::schema::{actors, follows};
use crate::POOL;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{sql_query, Insertable};
use diesel::{AsChangeset, Identifiable, Queryable};
use jdt_activity_pub::ApFollow;
use serde::{Deserialize, Serialize};

use super::actors::{get_actor_by_as_id, Actor};

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = follows)]
pub struct NewFollow {
    pub follower_ap_id: String,
    pub leader_ap_id: String,
    pub follow_activity_ap_id: Option<String>,
    pub accept_activity_ap_id: Option<String>,
    pub accepted: bool,
    pub follower_actor_id: Option<i32>,
    pub leader_actor_id: Option<i32>,
}

impl NewFollow {
    pub async fn link(mut self, conn: Option<&Db>) -> NewFollow {
        self.follower_actor_id = get_actor_by_as_id(conn, self.follower_ap_id.clone())
            .await
            .ok()
            .map(|x| x.id);

        self.leader_actor_id = get_actor_by_as_id(conn, self.leader_ap_id.clone())
            .await
            .ok()
            .map(|x| x.id);

        self.clone()
    }
}

impl TryFrom<ApFollow> for NewFollow {
    type Error = anyhow::Error;

    fn try_from(follow: ApFollow) -> Result<Self, Self::Error> {
        Ok(NewFollow {
            follower_ap_id: follow.actor.to_string(),
            leader_ap_id: follow
                .object
                .reference()
                .ok_or(anyhow!("ApFollow must have a Referenceable Object"))?,
            accepted: false,
            ..Default::default()
        })
    }
}

#[derive(
    Identifiable, Queryable, QueryableByName, AsChangeset, Serialize, Clone, Default, Debug,
)]
#[diesel(table_name = follows)]
pub struct Follow {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub follower_ap_id: String,
    pub leader_ap_id: String,
    pub follow_activity_ap_id: Option<String>,
    pub accept_activity_ap_id: Option<String>,
    pub accepted: bool,
    pub follower_actor_id: Option<i32>,
    pub leader_actor_id: Option<i32>,
}

pub async fn create_follow(conn: Option<&Db>, follower: NewFollow) -> Result<Follow> {
    let operation = move |c: &mut diesel::PgConnection| {
        diesel::insert_into(follows::table)
            .values(&follower)
            .get_result::<Follow>(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}

pub async fn delete_follower_by_ap_id(conn: Option<&Db>, ap_id: String) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        diesel::delete(follows::table)
            .filter(follows::follower_ap_id.eq(ap_id))
            .execute(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}

pub async fn get_followers_by_actor_id(
    conn_opt: Option<&Db>,
    actor_id: i32,
    paging: Option<OffsetPaging>,
) -> Vec<(Follow, Actor)> {
    // inner join is used to exclude Actor records that have been deleted
    let operation = move |c: &mut PgConnection| {
        let mut query = follows::table
            .filter(follows::leader_actor_id.eq(actor_id))
            .inner_join(actors::table.on(follows::follower_ap_id.eq(actors::as_id)))
            .order_by(follows::created_at.desc())
            .into_boxed();

        if let Some(paging) = paging {
            query = query
                .limit(paging.limit as i64)
                .offset((paging.page * paging.limit) as i64);
        }

        query
            .get_results::<(Follow, Actor)>(c)
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
        follows::table
            .filter(follows::leader_actor_id.eq(actor_id))
            .inner_join(actors::table.on(follows::follower_ap_id.eq(actors::as_id)))
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

        sql_query("DELETE FROM follow WHERE follower_ap_id COLLATE \"C\" LIKE $1")
            .bind::<Text, _>(format!("https://{domain_pattern}/%"))
            .execute(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}

pub async fn delete_followers_by_followed_ap_id(conn: Option<&Db>, ap_id: String) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("DELETE FROM follows WHERE leader_ap_id = $1")
            .bind::<Text, _>(ap_id)
            .execute(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}

pub async fn delete_followers_by_actor(conn: Option<&Db>, actor: String) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("DELETE FROM follows WHERE follower_ap_id = $1")
            .bind::<Text, _>(actor)
            .execute(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}

// Code from leaders, consolidated here.

pub async fn mark_follow_accepted(
    conn: Option<&Db>,
    follower_ap_id: String,
    leader_ap_id: String,
    accept_ap_id: String,
) -> Option<Follow> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("UPDATE follows SET accepted = 'true', accept_activity_ap_id = $1 WHERE follower_ap_id = $2 AND leader_ap_id = $3 RETURNING *")
            .bind::<Text, _>(accept_ap_id)
            .bind::<Text, _>(follower_ap_id)
            .bind::<Text, _>(leader_ap_id)
            .get_result(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation)
        .await
        .ok()
}

pub async fn get_follow(
    conn: Option<&Db>,
    follower_ap_id: String,
    leader_ap_id: String,
) -> Option<Follow> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("SELECT * FROM follows WHERE follower_ap_id = $1 AND leader_ap_id = $2")
            .bind::<Text, _>(follower_ap_id)
            .bind::<Text, _>(leader_ap_id)
            .get_result(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation)
        .await
        .ok()
}

pub async fn delete_follow(
    conn: Option<&Db>,
    follower_ap_id: String,
    leader_ap_id: String,
) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("DELETE FROM follows WHERE follower_ap_id = $1 AND leader_ap_id = $2")
            .bind::<Text, _>(follower_ap_id)
            .bind::<Text, _>(leader_ap_id)
            .execute(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}

pub async fn get_leaders_by_follower_actor_id(
    conn: Option<&Db>,
    follower_actor_id: i32,
    paging: Option<OffsetPaging>,
) -> Result<Vec<(Follow, Option<Actor>)>> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Integer;

        let mut offset = "".to_string();
        let mut limit = "".to_string();

        if let Some(paging) = paging {
            limit = format!(" LIMIT {}", paging.limit);
            offset = format!(" OFFSET {}", (paging.page * paging.limit));
        };

        sql_query(format!("SELECT * FROM follows f LEFT JOIN actors a ON (f.leader_ap_id = a.as_id) WHERE f.follower_actor_id = $1 ORDER BY f.created_at DESC{limit}{offset}"))
            .bind::<Integer, _>(follower_actor_id)
            .get_results::<(Follow, Option<Actor>)>(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}

pub async fn get_leader_count_by_follower_actor_id(
    conn: Option<&Db>,
    follower_actor_id: i32,
) -> Result<i64> {
    let operation = move |c: &mut diesel::PgConnection| {
        follows::table
            .filter(follows::follower_actor_id.eq(follower_actor_id))
            .count()
            .get_result(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}

pub async fn delete_follows_by_domain_pattern(
    conn: Option<&Db>,
    domain_pattern: String,
) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("DELETE FROM follows WHERE leader_ap_id COLLATE \"C\" LIKE $1 OR follower_ap_id COLLATE \"C\" LIKE $2")
            .bind::<Text, _>(format!("https://{domain_pattern}/%"))
            .bind::<Text, _>(format!("https://{domain_pattern}/%"))
            .execute(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}

pub async fn delete_follows_by_leader_ap_id(
    conn: Option<&Db>,
    leader_ap_id: String,
) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("DELETE FROM follows WHERE leader_ap_id = $1")
            .bind::<Text, _>(leader_ap_id)
            .execute(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}

pub async fn delete_follows_by_follower_ap_id(
    conn: Option<&Db>,
    follower_ap_id: String,
) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("DELETE FROM leaders WHERE follower_ap_id = $1")
            .bind::<Text, _>(follower_ap_id)
            .execute(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}
