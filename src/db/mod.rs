pub mod runner;

use anyhow::Result;
use diesel::{pg::PgConnection, result::QueryResult};
use rocket_sync_db_pools::{database, diesel};

// The name here ("diesel_postgres_pool") must match the database name in Rocket.toml
#[database("enigmatick")]
pub struct DbPool(pub diesel::PgConnection);

/// The database connection type used by Rocket routes.
pub type Db = DbPool;

/// The Diesel backend type.
pub type DbType = diesel::pg::Pg;

use deadpool_diesel::postgres::{Manager, Pool};
use once_cell::sync::Lazy;
use std::env;

/// A global connection pool used by background tasks or code paths
/// that don't have access to a request-scoped connection.
/// NOTE: For new code, prefer passing a `DbRunner` implementor instead of relying on this global.
pub static POOL: Lazy<Pool> = Lazy::new(|| {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
    Pool::builder(manager)
        .build()
        .expect("Failed to create global pool.")
});

// pub async fn run_db_op<F, T>(conn: Option<&Db>, pool: &Pool, operation: F) -> Result<T>
// where
//     F: FnOnce(&mut PgConnection) -> QueryResult<T> + Send + 'static,
//     T: Send + 'static,
// {
//     if let Some(conn) = conn {
//         // Using Rocket's connection from rocket_sync_db_pools
//         Ok(conn.run(operation).await??)
//     } else {
//         // Using the global deadpool_diesel pool
//         let conn = pool.get().await?;
//         match conn.interact(operation).await {
//             Ok(Ok(result)) => Ok(result),
//             Ok(Err(e)) => Err(e.into()),
//             Err(e) => Err(anyhow::anyhow!("DB interaction failed: {}", e)),
//         }
//     }
// }
