use crate::db::Db;
use crate::models::followers::NewFollower;
use crate::schema::followers;
use crate::POOL;
use diesel::prelude::*;

use chrono::NaiveDateTime;
use diesel::{AsChangeset, Identifiable, Queryable};
use serde::Serialize;

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = followers)]
pub struct Follower {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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
                    .execute(c)?;
                followers::table
                    .order(followers::id.desc())
                    .first::<Follower>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(followers::table)
                .values(&follower)
                .execute(&mut pool)
                .ok()?;
            followers::table
                .order(followers::id.desc())
                .first::<Follower>(&mut pool)
                .ok()
        }
    }
}
