pub mod runner;

use deadpool_diesel::postgres::{Manager, Pool};
use once_cell::sync::Lazy;
//use rocket_sync_db_pools::{database, diesel};
use diesel;
use std::env;

// The name here ("diesel_postgres_pool") must match the database name in Rocket.toml
//#[database("enigmatick")]
pub struct DbPool(pub diesel::PgConnection);

/// The Diesel backend type.
pub type DbType = diesel::pg::Pg;

/// A global connection pool used by background tasks or code paths
/// that don't have access to a request-scoped connection.
/// NOTE: For new code, prefer passing a `DbRunner` implementor instead of relying on this global.
pub static POOL: Lazy<Pool> = Lazy::new(|| {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
    Pool::builder(manager)
        // Limit pool size to reduce memory from libpq buffers (uses glibc malloc)
        // Each connection allocates ~1-2 MB of buffers that glibc may not return to OS
        .max_size(8) // Reasonable default for most workloads
        .build()
        .expect("Failed to create global pool.")
});
