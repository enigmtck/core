use crate::db::Db;
use crate::schema::instances;
use crate::POOL;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

impl From<String> for NewInstance {
    fn from(domain_name: String) -> Self {
        NewInstance {
            domain_name,
            json: None,
            blocked: false,
        }
    }
}

pub async fn create_instance(conn: Option<&Db>, instance: NewInstance) -> Option<Instance> {
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
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(instances::table)
                .values(&instance)
                .on_conflict(instances::domain_name)
                .do_update()
                .set(instances::last_message_at.eq(Utc::now().naive_utc()))
                .execute(&mut pool)
                .ok()?;
            instances::table
                .filter(instances::domain_name.eq(&instance.domain_name))
                .first::<Instance>(&mut pool)
                .ok()
        }
    }
}

pub async fn get_instance_by_domain_name(conn: &Db, domain_name: String) -> Option<Instance> {
    conn.run(move |c| {
        instances::table
            .filter(instances::domain_name.eq(domain_name))
            .first::<Instance>(c)
    })
    .await
    .ok()
}

pub async fn get_blocked_instances(conn: Option<&Db>) -> Vec<Instance> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                instances::table
                    .filter(instances::blocked.eq(true))
                    .get_results::<Instance>(c)
            })
            .await
            .unwrap_or(vec![]),
        None => {
            if let Ok(mut pool) = POOL.get() {
                instances::table
                    .filter(instances::blocked.eq(true))
                    .get_results::<Instance>(&mut pool)
                    .unwrap_or(vec![])
            } else {
                vec![]
            }
        }
    }
}
