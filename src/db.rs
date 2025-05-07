use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use rocket_sync_db_pools::database;
use anyhow::Result;
use log;
use tokio::task::spawn_blocking;
// Remove: use diesel::Connection as DieselConnection; // No longer needed for the generic C

// Macro to define run_db_op for a specific connection type
macro_rules! define_run_db_op {
    ($conn_type:ty) => {
        pub async fn run_db_op<F, R>(
            db_opt: Option<&Db>, // Db and Pool types are from the cfg_if! block's scope
            pool_ref: &Pool,
            operation: F,
        ) -> Result<R>
        where
            F: FnOnce(&mut $conn_type) -> diesel::QueryResult<R> + Send + 'static,
            R: Send + 'static,
        {
            if let Some(db_conn_wrapper) = db_opt {
                // .0 accesses the rocket_sync_db_pools::Connection wrapper inside your Db struct
                // This wrapper's run method expects a closure taking &mut $conn_type
                db_conn_wrapper.0.run(operation).await.map_err(|diesel_err| {
                    match diesel_err {
                        diesel::result::Error::NotFound => {
                            log::debug!(
                                "Db operation reported NotFound (Rocket Db wrapper): {:?}",
                                diesel_err
                            );
                        }
                        _ => {
                            log::error!(
                                "Error executing Db operation via Rocket Db wrapper: {:?}",
                                diesel_err
                            );
                        }
                    }
                    anyhow::anyhow!("Db operation failed (Rocket Db wrapper): {}", diesel_err)
                })
            } else {
                let pool_clone = pool_ref.clone(); // r2d2::Pool is cheap to clone (Arc-based)
                spawn_blocking(move || {
                    let mut pooled_conn = pool_clone.get().map_err(|e| {
                        log::error!("Failed to get connection from POOL: {:?}", e);
                        anyhow::anyhow!("Failed to get pooled connection: {}", e)
                    })?;
                    // Dereferencing r2d2::PooledConnection gives $conn_type
                    operation(&mut *pooled_conn).map_err(|diesel_err| {
                        match diesel_err {
                            diesel::result::Error::NotFound => {
                                log::debug!("Db operation reported NotFound (POOL): {:?}", diesel_err);
                            }
                            _ => {
                                log::error!("Error executing Db operation via POOL: {:?}", diesel_err);
                            }
                        }
                        anyhow::anyhow!("DB operation failed (POOL): {}", diesel_err)
                    })
                })
                .await
                .map_err(|e| { // Handles JoinError if spawn_blocking panics or is cancelled
                    log::error!("Task for Db operation via POOL failed: {:?}", e);
                    anyhow::anyhow!("Db task failed (POOL): {}", e)
                })? // This '?' unwraps the JoinError Result, then the inner Result<R, anyhow::Error>
            }
        }
    };
}

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        #[database("enigmatick")]
        pub struct Db(PgConnection); // Generated Db is effectively Db(rocket_sync_db_pools::Connection<PgConnection>)
        pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
        pub type PooledConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

        // Generate run_db_op specifically for PgConnection
        define_run_db_op!(PgConnection);

    } else if #[cfg(feature = "sqlite")] {
        #[database("enigmatick")]
        pub struct Db(SqliteConnection); // Generated Db is effectively Db(rocket_sync_db_pools::Connection<SqliteConnection>)
        pub type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;
        pub type PooledConnection = r2d2::PooledConnection<ConnectionManager<SqliteConnection>>; // Ensure this is defined

        // Generate run_db_op specifically for SqliteConnection
        define_run_db_op!(SqliteConnection);
    }
}
