use crate::db::Db;
use crate::schema::instances;
use crate::POOL;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = instances)]
pub struct NewInstance {
    pub domain_name: String,
    pub json: Option<Value>,
    pub blocked: bool,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = instances)]
pub struct Instance {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub domain_name: String,
    pub json: Option<Value>,
    pub blocked: bool,
    pub last_message_at: DateTime<Utc>,
}

pub async fn create_or_update_instance(
    conn: Option<&Db>,
    instance: NewInstance,
) -> Result<Instance, anyhow::Error> {
    match conn {
        Some(conn) => {
            conn.run(move |c| {
                diesel::insert_into(instances::table)
                    .values(&instance)
                    .on_conflict(instances::domain_name)
                    .do_update()
                    .set(instances::last_message_at.eq(Utc::now()))
                    .get_result::<Instance>(c)
                    .map_err(anyhow::Error::msg)
            })
            .await
        }
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            diesel::insert_into(instances::table)
                .values(&instance)
                .on_conflict(instances::domain_name)
                .do_update()
                .set(instances::last_message_at.eq(Utc::now()))
                .get_result::<Instance>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}
