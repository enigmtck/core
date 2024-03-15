use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use rocket_sync_db_pools::database;

// this is a reference to the value stored in env, not the actual
// database name
#[database("enigmatick")]
pub struct Db(PgConnection);

#[cfg(feature = "pg")]
pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[cfg(feature = "pg")]
pub type PooledConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;
