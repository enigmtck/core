use crate::db::Db;
use crate::models::remote_actor_hashtags::NewRemoteActorHashtag;
use crate::schema::remote_actor_hashtags;
use crate::POOL;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::Serialize;

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = remote_actor_hashtags)]
pub struct RemoteActorHashtag {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub hashtag: String,
    pub remote_actor_id: i32,
}

pub async fn create_remote_actor_hashtag(
    conn: Option<&Db>,
    hashtag: NewRemoteActorHashtag,
) -> Option<RemoteActorHashtag> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(remote_actor_hashtags::table)
                    .values(&hashtag)
                    .get_result::<RemoteActorHashtag>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(remote_actor_hashtags::table)
                .values(&hashtag)
                .get_result::<RemoteActorHashtag>(&mut pool)
                .ok()
        }
    }
}
