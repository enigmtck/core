use crate::db::Db;
use crate::models::followers::NewFollower;
use crate::schema::followers;
use crate::POOL;
use diesel::prelude::*;

use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Queryable};
use serde::Serialize;

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
    pub actor_id: i32,
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
