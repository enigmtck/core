use crate::activity_pub::{ApActor, ApTag};
use crate::db::Db;
use crate::schema::remote_actor_hashtags;
use crate::POOL;
use anyhow::Result;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

use super::remote_actors::RemoteActor;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = remote_actor_hashtags)]
pub struct NewRemoteActorHashtag {
    pub hashtag: String,
    pub remote_actor_id: i32,
}

impl From<RemoteActor> for Vec<NewRemoteActorHashtag> {
    fn from(remote_actor: RemoteActor) -> Self {
        let ap_actor: ApActor = remote_actor.clone().into();

        ap_actor
            .tag
            .unwrap_or_default()
            .iter()
            .filter_map(|tag| {
                if let ApTag::HashTag(tag) = tag {
                    Some(NewRemoteActorHashtag {
                        hashtag: tag.name.clone(),
                        remote_actor_id: remote_actor.id,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = remote_actor_hashtags)]
pub struct RemoteActorHashtag {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub hashtag: String,
    pub remote_actor_id: i32,
}

pub async fn create_remote_actor_hashtag(
    conn: Option<&Db>,
    hashtag: NewRemoteActorHashtag,
) -> Option<RemoteActorHashtag> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(remote_actor_hashtags::table)
                    .values(&hashtag)
                    .execute(c)
            })
            .await
            .ok()?;

            conn.run(move |c| {
                remote_actor_hashtags::table
                    .order(remote_actor_hashtags::id.desc())
                    .first::<RemoteActorHashtag>(c)
            })
            .await
            .ok()
        }
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(remote_actor_hashtags::table)
                .values(&hashtag)
                .execute(&mut pool)
                .ok()?;

            remote_actor_hashtags::table
                .order(remote_actor_hashtags::id.desc())
                .first::<RemoteActorHashtag>(&mut pool)
                .ok()
        }
    }
}

pub async fn delete_remote_actor_hashtags_by_remote_actor_id(
    conn: Option<&Db>,
    id: i32,
) -> Result<usize> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::delete(
                    remote_actor_hashtags::table
                        .filter(remote_actor_hashtags::remote_actor_id.eq(id)),
                )
                .execute(c)
                .map_err(anyhow::Error::msg)
            })
            .await
        }
        None => {
            let mut pool = POOL.get()?;
            diesel::delete(
                remote_actor_hashtags::table.filter(remote_actor_hashtags::remote_actor_id.eq(id)),
            )
            .execute(&mut pool)
            .map_err(anyhow::Error::msg)
        }
    }
}
