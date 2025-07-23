use anyhow::Result;
use diesel::pg::PgConnection;
use diesel::result::QueryResult;
use std::future::Future;
use std::pin::Pin;

pub trait DbRunner: Send + Sync {
    fn run<F, T>(&self, f: F) -> Pin<Box<dyn Future<Output = Result<T>> + Send + '_>>
    where
        F: FnOnce(&mut PgConnection) -> QueryResult<T> + Send + 'static,
        T: Send + 'static;
}

impl DbRunner for deadpool_diesel::postgres::Object {
    fn run<F, T>(&self, f: F) -> Pin<Box<dyn Future<Output = Result<T>> + Send + '_>>
    where
        F: FnOnce(&mut PgConnection) -> QueryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        Box::pin(async move {
            match self.interact(f).await {
                Ok(Ok(result)) => Ok(result),
                Ok(Err(e)) => Err(e.into()),
                Err(e) => Err(anyhow::anyhow!("DB interaction failed: {}", e)),
            }
        })
    }
}
