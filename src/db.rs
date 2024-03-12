use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use rocket_sync_db_pools::database;

// this is a reference to the value stored in env, not the actual
// database name
#[database("enigmatick")]
pub struct Db(SqliteConnection);

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type SqlitePool = r2d2::Pool<ConnectionManager<SqliteConnection>>;
pub type PooledConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;
