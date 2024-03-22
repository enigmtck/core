use crate::db::Db;
use crate::schema::processing_queue;
use crate::POOL;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = processing_queue)]
pub struct NewProcessingItem {
    pub profile_id: i32,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: String,
    pub cc: Option<String>,
    pub attributed_to: String,
    pub ap_object: String,
    pub processed: bool,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = processing_queue)]
pub struct ProcessingItem {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub ap_id: String,
    pub ap_to: String,
    pub cc: Option<String>,
    pub attributed_to: String,
    pub kind: String,
    pub ap_object: String,
    pub processed: bool,
    pub profile_id: i32,
}

pub async fn create_processing_item(
    conn: Option<&Db>,
    processing_item: NewProcessingItem,
) -> Option<ProcessingItem> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(processing_queue::table)
                    .values(&processing_item)
                    .execute(c)
            })
            .await
            .ok()?;

            conn.run(move |c| {
                processing_queue::table
                    .order(processing_queue::id.desc())
                    .first::<ProcessingItem>(c)
            })
            .await
            .ok()
        }
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(processing_queue::table)
                .values(&processing_item)
                .execute(&mut pool)
                .ok()?;

            processing_queue::table
                .order(processing_queue::id.desc())
                .first::<ProcessingItem>(&mut pool)
                .ok()
        }
    }
}

pub async fn resolve_processed_item_by_ap_id_and_profile_id(
    conn: &Db,
    profile_id: i32,
    ap_id: String,
) -> Option<ProcessingItem> {
    conn.run(move |c| {
        diesel::update(
            processing_queue::table
                .filter(processing_queue::ap_id.eq(ap_id.clone()))
                .filter(processing_queue::profile_id.eq(profile_id)),
        )
        .set(processing_queue::processed.eq(true))
        .execute(c)
        .ok()?;

        processing_queue::table
            .filter(processing_queue::ap_id.eq(ap_id))
            .filter(processing_queue::profile_id.eq(profile_id))
            .first::<ProcessingItem>(c)
            .ok()
    })
    .await
}
