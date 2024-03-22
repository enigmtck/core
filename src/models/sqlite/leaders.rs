use crate::db::Db;
use crate::models::leaders::NewLeader;
use crate::schema::leaders;
use crate::POOL;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use serde::Serialize;

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = leaders)]
pub struct Leader {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub profile_id: i32,
    pub actor: String,
    pub leader_ap_id: String,
    pub uuid: String,
    pub accept_ap_id: Option<String>,
    pub accepted: Option<bool>,
    pub follow_ap_id: Option<String>,
}

pub async fn create_leader(conn: Option<&Db>, leader: NewLeader) -> Option<Leader> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(leaders::table)
                    .values(&leader)
                    .execute(c)
                    .ok()?;

                leaders::table
                    .order(leaders::id.desc())
                    .first::<Leader>(c)
                    .ok()
            })
            .await
        }
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(leaders::table)
                .values(&leader)
                .execute(&mut pool)
                .ok()?;

            leaders::table
                .order(leaders::id.desc())
                .first::<Leader>(&mut pool)
                .ok()
        }
    }
}

pub async fn update_leader_by_uuid(
    conn: &Db,
    leader_uuid: String,
    accept_ap_id: String,
) -> Option<Leader> {
    let leader_uuid_clone = leader_uuid.clone();
    conn.run(move |c| {
        diesel::update(leaders::table.filter(leaders::uuid.eq(&leader_uuid_clone)))
            .set((
                leaders::accept_ap_id.eq(&accept_ap_id),
                leaders::accepted.eq(true),
            ))
            .execute(c)
    })
    .await
    .ok()?;

    conn.run(move |c| {
        leaders::table
            .filter(leaders::uuid.eq(leader_uuid))
            .first::<Leader>(c)
    })
    .await
    .ok()
}
