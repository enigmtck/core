use crate::activity_pub::{ApActor, ApTag};
use crate::db::Db;
use crate::schema::remote_actor_hashtags;
use crate::POOL;
use anyhow::Result;
use diesel::prelude::*;
use diesel::Insertable;
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

use super::remote_actors::RemoteActor;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::remote_actor_hashtags::RemoteActorHashtag;
        pub use crate::models::pg::remote_actor_hashtags::create_remote_actor_hashtag;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::remote_actor_hashtags::RemoteActorHashtag;
        pub use crate::models::sqlite::remote_actor_hashtags::create_remote_actor_hashtag;
    }
}

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
