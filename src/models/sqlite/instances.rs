use crate::db::Db;
use crate::schema::instances;
use crate::POOL;
use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = instances)]
pub struct NewInstance {
    pub domain_name: String,
    pub json: Option<String>,
    pub blocked: bool,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = instances)]
pub struct Instance {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub domain_name: String,
    pub json: Option<String>,
    pub blocked: bool,
    pub last_message_at: NaiveDateTime,
}

pub async fn create_or_update_instance(
    conn: Option<&Db>,
    instance: NewInstance,
) -> Result<Instance, anyhow::Error> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(instances::table)
                    .values(&instance)
                    .on_conflict(instances::domain_name)
                    .do_update()
                    .set(instances::last_message_at.eq(Utc::now().naive_utc()))
                    .execute(c)?;

                instances::table
                    .filter(instances::domain_name.eq(&instance.domain_name))
                    .first::<Instance>(c)
            })
            .await
            .map_err(anyhow::Error::msg),
        None => {
            let mut pool = POOL.get().map_err(anyhow::Error::msg)?;
            diesel::insert_into(instances::table)
                .values(&instance)
                .on_conflict(instances::domain_name)
                .do_update()
                .set(instances::last_message_at.eq(Utc::now().naive_utc()))
                .execute(&mut pool)?;

            instances::table
                .filter(instances::domain_name.eq(&instance.domain_name))
                .first::<Instance>(&mut pool)
                .map_err(anyhow::Error::msg)
        }
    }
}
