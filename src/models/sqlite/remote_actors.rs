use crate::activity_pub::ApContext;
use crate::db::Db;
use crate::schema::remote_actors;
use crate::POOL;
use anyhow::Result;
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

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
    pub checked_at: NaiveDateTime,
    pub webfinger: Option<String>,
}

impl RemoteActor {
    pub fn is_stale(&self) -> bool {
        Utc::now().naive_local() - self.updated_at > Duration::days(7)
    }
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
