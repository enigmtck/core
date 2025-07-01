use super::OffsetPaging;
use crate::db::Db;
use crate::helper::{get_local_identifier, LocalIdentifierType};
use crate::schema::{actors, leaders};
use crate::POOL;
use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{sql_query, Insertable};
use diesel::{AsChangeset, Identifiable, Queryable};
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{ApAccept, ApActivity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::actors::Actor;

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = leaders)]
pub struct Leader {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub actor: String,
    pub leader_ap_id: String,
    pub uuid: String,
    pub accept_ap_id: Option<String>,
    pub accepted: Option<bool>,
    pub follow_ap_id: Option<String>,
    pub actor_id: i32,
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = leaders)]
pub struct NewLeader {
    pub actor: String,
    pub leader_ap_id: String,
    pub uuid: String,
    pub accept_ap_id: Option<String>,
    pub accepted: Option<bool>,
    pub follow_ap_id: Option<String>,
    pub actor_id: i32,
}

impl TryFrom<ApAccept> for NewLeader {
    type Error = &'static str;

    fn try_from(accept: ApAccept) -> Result<Self, Self::Error> {
        if let MaybeReference::Actual(ApActivity::Follow(follow)) = accept.object {
            Ok(NewLeader {
                actor: follow.actor.to_string(),
                leader_ap_id: accept.actor.to_string(),
                uuid: Uuid::new_v4().to_string(),
                accept_ap_id: accept.id,
                accepted: Some(true),
                follow_ap_id: follow.id,
                actor_id: -1,
            })
        } else {
            Err("ACCEPT DOES NOT CONTAIN A VALID FOLLOW OBJECT")
        }
    }
}

impl NewLeader {
    pub fn link(&mut self, actor: Actor) -> Self {
        if let Some(id) = get_local_identifier(self.actor.clone()) {
            if id.kind == LocalIdentifierType::User
                && id.identifier.to_lowercase() == actor.ek_username.unwrap().to_lowercase()
            {
                self.actor_id = actor.id;
                self.clone()
            } else {
                self.clone()
            }
        } else {
            self.clone()
        }
    }
}

pub async fn create_leader(conn: Option<&Db>, leader: NewLeader) -> Option<Leader> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(leaders::table)
                    .values(&leader)
                    .get_result::<Leader>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(leaders::table)
                .values(&leader)
                .get_result::<Leader>(&mut pool)
                .ok()
        }
    }
}

pub async fn update_leader_by_uuid(
    conn: &Db,
    leader_uuid: String,
    accept_ap_id: String,
) -> Option<Leader> {
    conn.run(move |c| {
        diesel::update(leaders::table.filter(leaders::uuid.eq(leader_uuid)))
            .set((
                leaders::accept_ap_id.eq(accept_ap_id),
                leaders::accepted.eq(true),
            ))
            .get_result::<Leader>(c)
    })
    .await
    .ok()
}

pub async fn get_leader_by_actor_ap_id_and_profile(
    conn: &crate::db::Db,
    ap_id: String,
    actor_id: i32,
) -> Option<Leader> {
    conn.run(move |c| {
        leaders::table
            .filter(leaders::leader_ap_id.eq(ap_id))
            .filter(leaders::actor_id.eq(actor_id))
            .first::<Leader>(c)
    })
    .await
    .ok()
}

pub async fn delete_leader(conn: &Db, leader_id: i32) -> bool {
    conn.run(move |c| diesel::delete(leaders::table.find(leader_id)).execute(c))
        .await
        .is_ok()
}

pub async fn delete_leader_by_ap_id_and_actor_id(
    conn: Option<&Db>,
    leader_ap_id: String,
    actor_id: i32,
) -> bool {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::delete(
                    leaders::table
                        .filter(leaders::actor_id.eq(actor_id))
                        .filter(leaders::leader_ap_id.eq(leader_ap_id)),
                )
                .execute(c)
            })
            .await
            .is_ok(),
        None => POOL.get().is_ok_and(|mut pool| {
            diesel::delete(
                leaders::table
                    .filter(leaders::actor_id.eq(actor_id))
                    .filter(leaders::leader_ap_id.eq(leader_ap_id)),
            )
            .execute(&mut pool)
            .is_ok()
        }),
    }
}

pub async fn get_leader_by_actor_id_and_ap_id(
    conn: &Db,
    actor_id: i32,
    leader_ap_id: String,
) -> Option<Leader> {
    conn.run(move |c| {
        leaders::table
            .filter(
                leaders::actor_id
                    .eq(actor_id)
                    .and(leaders::leader_ap_id.eq(leader_ap_id)),
            )
            .first::<Leader>(c)
    })
    .await
    .ok()
}

pub async fn get_leaders_by_actor_id(
    conn: &Db,
    actor_id: i32,
    paging: Option<OffsetPaging>,
) -> Vec<(Leader, Option<Actor>)> {
    conn.run(move |c| {
        let mut query = leaders::table
            .filter(leaders::actor_id.eq(actor_id))
            .left_join(actors::table.on(leaders::leader_ap_id.eq(actors::as_id)))
            .order_by(leaders::created_at.desc())
            .into_boxed();

        if let Some(paging) = paging {
            query = query
                .limit(paging.limit as i64)
                .offset((paging.page * paging.limit) as i64);
        }

        query.get_results::<(Leader, Option<Actor>)>(c)
    })
    .await
    .unwrap_or(vec![])
}

pub async fn get_leader_count_by_actor_id(conn: &Db, actor_id: i32) -> Result<i64> {
    conn.run(move |c| {
        leaders::table
            .filter(leaders::actor_id.eq(actor_id))
            .count()
            .get_result(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn delete_leaders_by_domain_pattern(
    conn: Option<&Db>,
    domain_pattern: String,
) -> Result<usize> {
    let operation = move |c: &mut diesel::PgConnection| {
        use diesel::sql_types::Text;

        sql_query("DELETE FROM leaders WHERE leader_ap_id COLLATE \"C\" LIKE $1")
            .bind::<Text, _>(format!("https://{}/%", domain_pattern))
            .execute(c)
    };

    crate::db::run_db_op(conn, &crate::POOL, operation).await
}
