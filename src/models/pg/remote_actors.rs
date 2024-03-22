use crate::db::Db;
use crate::schema::remote_actors;
use crate::POOL;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, AsChangeset)]
#[diesel(table_name = remote_actors)]
pub struct NewRemoteActor {
    pub context: Value,
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
    pub public_key: Value,
    pub featured: Option<String>,
    pub featured_tags: Option<String>,
    pub url: Option<String>,
    pub manually_approves_followers: Option<bool>,
    pub published: Option<String>,
    pub tag: Option<Value>,
    pub attachment: Option<Value>,
    pub endpoints: Option<Value>,
    pub icon: Option<Value>,
    pub image: Option<Value>,
    pub also_known_as: Option<Value>,
    pub discoverable: Option<bool>,
    pub capabilities: Option<Value>,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Debug)]
#[diesel(table_name = remote_actors)]
pub struct RemoteActor {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub context: Value,
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
    pub public_key: Option<Value>,
    pub featured: Option<String>,
    pub featured_tags: Option<String>,
    pub url: Option<String>,
    pub manually_approves_followers: Option<bool>,
    pub published: Option<String>,
    pub tag: Option<Value>,
    pub attachment: Option<Value>,
    pub endpoints: Option<Value>,
    pub icon: Option<Value>,
    pub image: Option<Value>,
    pub also_known_as: Option<Value>,
    pub discoverable: Option<bool>,
    pub capabilities: Option<Value>,
    pub checked_at: DateTime<Utc>,
    pub webfinger: Option<String>,
}

impl RemoteActor {
    pub fn is_stale(&self) -> bool {
        Utc::now() - self.updated_at > Duration::days(7)
    }
}

pub async fn create_or_update_remote_actor(
    conn: Option<&Db>,
    actor: NewRemoteActor,
) -> Result<RemoteActor> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(remote_actors::table)
                    .values(&actor)
                    .on_conflict(remote_actors::ap_id)
                    .do_update()
                    .set((&actor, remote_actors::checked_at.eq(Utc::now())))
                    .get_result::<RemoteActor>(c)
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
                .set((&actor, remote_actors::checked_at.eq(Utc::now())))
                .get_result::<RemoteActor>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}
