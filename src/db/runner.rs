use crate::db::Db;
use anyhow::Result;
use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;

#[async_trait]
pub trait DbRunner: Send + Sync {
    async fn run<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut PgConnection) -> QueryResult<T> + Send + 'static,
        T: Send + 'static;
}

#[async_trait]
impl DbRunner for Db {
    async fn run<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut PgConnection) -> QueryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        // self.run(f).await returns a Result<T, rocket_sync_db_pools::Error>
        // where T is the success type from the closure `f`.
        // The closure itself returns a QueryResult<T>, which is Result<T, diesel::Error>.
        // The `run` method from rocket_sync_db_pools handles the inner QueryResult for us.
        // We only need to handle the pool's error type.
        self.run(f).await.map_err(|e| {
            // The error `e` is of type rocket_sync_db_pools::Error.
            // We convert it into an anyhow::Error.
            anyhow::anyhow!("Rocket DB pool error: {:?}", e)
        })
    }
}

#[async_trait]
impl DbRunner for &Db {
    async fn run<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut PgConnection) -> QueryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        // self.run(f).await returns a Result<T, rocket_sync_db_pools::Error>
        // where T is the success type from the closure `f`.
        // The closure itself returns a QueryResult<T>, which is Result<T, diesel::Error>.
        // The `run` method from rocket_sync_db_pools handles the inner QueryResult for us.
        // We only need to handle the pool's error type.
        (*self).run(f).await.map_err(|e| {
            // The error `e` is of type rocket_sync_db_pools::Error.
            // We convert it into an anyhow::Error.
            anyhow::anyhow!("Rocket DB pool error: {:?}", e)
        })
    }
}

#[async_trait]
impl DbRunner for deadpool_diesel::postgres::Object {
    async fn run<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut PgConnection) -> QueryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        match self.interact(f).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(e.into()),
            Err(e) => Err(anyhow::anyhow!("DB interaction failed: {}", e)),
        }
    }
}
