use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use rocket_sync_db_pools::database;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        #[database("enigmatick")]
        pub struct Db(PgConnection);
        pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
        pub type PooledConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;
    } else if #[cfg(feature = "sqlite")] {
        #[database("enigmatick")]
        pub struct Db(SqliteConnection);
        pub type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;
    }
}
