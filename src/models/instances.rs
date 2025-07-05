use crate::db::Db;
use crate::db::DbType;
use crate::schema::instances;
use crate::schema::instances::dsl;
use crate::POOL;
use chrono::{DateTime, Utc};
use deadpool_diesel::postgres::Object as DbConnection;
use diesel::prelude::*;
use diesel::OptionalExtension;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use jdt_activity_pub::ApAddress;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// --- Sort-related types ---
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    DomainName,
    Blocked,
    LastMessageAt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub struct SortParam {
    pub field: SortField,
    pub direction: SortDirection,
}
// --- End of Sort-related types ---

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

pub async fn get_instance_inboxes(conn_opt: Option<&Db>) -> Result<Vec<ApAddress>, anyhow::Error> {
    let operation = move |c: &mut PgConnection| {
        let cutoff = Utc::now().naive_utc() - chrono::Duration::days(14);
        instances::table
            .filter(instances::blocked.eq(false))
            .filter(instances::shared_inbox.is_not_null())
            .filter(instances::last_message_at.gt(cutoff))
            .select(instances::shared_inbox.assume_not_null())
            .get_results::<String>(c)
            .map_err(anyhow::Error::from)
    };

    let results = match conn_opt {
        Some(conn) => conn.run(operation).await?,
        None => {
            tokio::task::spawn_blocking(move || {
                let mut pool_conn = POOL.get().map_err(anyhow::Error::msg)?;
                operation(&mut pool_conn)
            })
            .await?? // First ? for JoinError, second for the operation's Result
        }
    };

    Ok(results.into_iter().map(ApAddress::from).collect())
}

pub async fn get_instance_by_domain_name(
    conn_opt: Option<&Db>,
    domain_name_val: String,
) -> Result<Option<Instance>, anyhow::Error> {
    let query = move |c: &mut _| {
        // Type of c will be inferred by conn.run()
        dsl::instances
            .filter(dsl::domain_name.eq(domain_name_val))
            .first::<Instance>(c)
            .optional()
    };

    if let Some(conn) = conn_opt {
        conn.run(query).await.map_err(anyhow::Error::from)
    } else {
        let mut pool_conn = POOL
            .get()
            .map_err(|e| anyhow::anyhow!("Failed to get DB connection: {e}"))?;
        query(&mut pool_conn).map_err(anyhow::Error::from)
    }
}

// Add this new function:
pub async fn get_all_instances_paginated(
    conn_opt: Option<&Db>,
    page: i64,
    page_size: i64,
    sort_params: Option<Vec<SortParam>>,
) -> Result<Vec<Instance>, anyhow::Error> {
    use crate::schema::instances::BoxedQuery;
    let offset = (page - 1).max(0) * page_size;

    let query_builder_fn = move |c: &mut _| {
        let mut query: BoxedQuery<'_, DbType> = dsl::instances.into_boxed();

        if let Some(params) = sort_params {
            if !params.is_empty() {
                for p in params {
                    query = match p.field {
                        SortField::DomainName => match p.direction {
                            SortDirection::Asc => query.then_order_by(dsl::domain_name.asc()),
                            SortDirection::Desc => query.then_order_by(dsl::domain_name.desc()),
                        },
                        SortField::Blocked => match p.direction {
                            SortDirection::Asc => query.then_order_by(dsl::blocked.asc()),
                            SortDirection::Desc => query.then_order_by(dsl::blocked.desc()),
                        },
                        SortField::LastMessageAt => match p.direction {
                            SortDirection::Asc => query.then_order_by(dsl::last_message_at.asc()),
                            SortDirection::Desc => query.then_order_by(dsl::last_message_at.desc()),
                        },
                    };
                }
            } else {
                // Default sort if params is Some but empty (shouldn't happen with current CLI parsing)
                // or if you want a specific default when --sort is provided but empty.
                query = query.order(dsl::domain_name.asc());
            }
        } else {
            // Default sort if no --sort argument is provided at all
            query = query.order(dsl::domain_name.asc());
        }

        query.limit(page_size).offset(offset).load::<Instance>(c)
    };

    if let Some(conn) = conn_opt {
        conn.run(query_builder_fn)
            .await
            .map_err(anyhow::Error::from)
    } else {
        let mut pool_conn = POOL
            .get()
            .map_err(|e| anyhow::anyhow!("Failed to get DB connection: {e}"))?;
        query_builder_fn(&mut pool_conn).map_err(anyhow::Error::from)
    }
}

pub async fn set_block_status(
    conn_opt: Option<&Db>,
    domain_name_val: String,
    should_be_blocked: bool,
) -> Result<Instance, anyhow::Error> {
    // Try to fetch the instance first.
    // We pass conn_opt here, so it correctly uses the provided transaction or a new pool connection.
    match get_instance_by_domain_name(conn_opt, domain_name_val.clone()).await {
        Ok(Some(instance)) => {
            if instance.blocked == should_be_blocked {
                Ok(instance)
            } else {
                let query_update = move |c: &mut _| {
                    diesel::update(dsl::instances.find(instance.id))
                        .set((
                            dsl::blocked.eq(should_be_blocked),
                            dsl::updated_at.eq(Utc::now()),
                        ))
                        .get_result::<Instance>(c)
                };
                if let Some(conn) = conn_opt {
                    conn.run(query_update).await.map_err(anyhow::Error::from)
                } else {
                    let mut pool_conn = POOL
                        .get()
                        .map_err(|e| anyhow::anyhow!("Failed to get DB connection: {e}"))?;
                    query_update(&mut pool_conn).map_err(anyhow::Error::from)
                }
            }
        }
        Ok(None) => {
            // Instance does not exist
            if should_be_blocked {
                // Create new instance with specified block status
                let new_instance_data = NewInstance {
                    domain_name: domain_name_val,
                    blocked: true,
                    json: None,
                    shared_inbox: None,
                };

                // This query will insert or, if a race condition occurred and it now exists, update.
                let query_upsert = move |c: &mut _| {
                    diesel::insert_into(dsl::instances)
                        .values(&new_instance_data)
                        .on_conflict(dsl::domain_name)
                        .do_update()
                        .set((
                            dsl::blocked.eq(true), // Explicitly set true for clarity
                            dsl::updated_at.eq(Utc::now()),
                            // Note: last_message_at is NOT touched here for existing records.
                            // For new records, it gets the DB default.
                        ))
                        .get_result::<Instance>(c)
                };
                if let Some(conn) = conn_opt {
                    conn.run(query_upsert).await.map_err(anyhow::Error::from)
                } else {
                    let mut pool_conn = POOL
                        .get()
                        .map_err(|e| anyhow::anyhow!("Failed to get DB connection: {e}"))?;
                    query_upsert(&mut pool_conn).map_err(anyhow::Error::from)
                }
            } else {
                // Trying to unblock a non-existent instance
                Err(anyhow::anyhow!(
                    "Instance {domain_name_val} not found. Cannot unblock a non-existent instance."
                ))
            }
        }
        Err(e) => {
            // Error during the initial get_instance_by_domain_name
            Err(e)
        }
    }
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

pub async fn create_or_update_instance_axum(
    conn: &DbConnection,
    instance: NewInstance,
) -> Result<Instance, anyhow::Error> {
    conn.interact(move |c| {
        use crate::schema::instances::dsl::*;
        diesel::insert_into(instances)
            .values(&instance)
            .on_conflict(domain_name)
            .do_update()
            .set((last_message_at.eq(Utc::now()), &instance))
            .get_result::<Instance>(c)
    })
    .await
    .map_err(|e| anyhow::anyhow!("Interact error: {:?}", e))?
    .map_err(anyhow::Error::from)
}
