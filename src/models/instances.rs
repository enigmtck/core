use crate::activity_pub::ApAddress;
use crate::db::Db;
use crate::schema::instances;
use crate::POOL;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, AsChangeset, Default, Debug, Clone)]
#[diesel(table_name = instances)]
pub struct NewInstance {
    pub domain_name: String,
    pub json: Option<Value>,
    pub blocked: bool,
    pub shared_inbox: Option<String>,
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
    pub shared_inbox: Option<String>,
}

pub async fn create_or_update_instance(
    conn: &Db,
    instance: NewInstance,
) -> Result<Instance, anyhow::Error> {
    conn.run(move |c| {
        diesel::insert_into(instances::table)
            .values(&instance)
            .on_conflict(instances::domain_name)
            .do_update()
            .set((instances::last_message_at.eq(Utc::now()), &instance))
            .get_result::<Instance>(c)
            .map_err(anyhow::Error::msg)
    })
    .await
}

pub type DomainInbox = (String, Option<String>);
impl From<DomainInbox> for NewInstance {
    fn from((domain_name, shared_inbox): DomainInbox) -> Self {
        NewInstance {
            domain_name,
            json: None,
            blocked: false,
            shared_inbox,
        }
    }
}

pub async fn get_instance_inboxes(conn: &Db) -> Vec<ApAddress> {
    let cutoff = Utc::now().naive_utc() - chrono::Duration::days(14);

    conn.run(move |c| {
        instances::table
            .filter(instances::blocked.eq(false))
            .filter(instances::shared_inbox.is_not_null())
            .filter(instances::last_message_at.gt(cutoff))
            .select(instances::shared_inbox.assume_not_null())
            .get_results::<String>(c)
    })
    .await
    .unwrap_or_default()
    .into_iter()
    .map(ApAddress::from)
    .collect()
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
