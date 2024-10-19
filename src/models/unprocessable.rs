use crate::db::Db;
use crate::schema::unprocessable;
use diesel::prelude::*;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::unprocessable::Unprocessable;
        pub use crate::models::pg::unprocessable::NewUnprocessable;
        pub use crate::models::pg::unprocessable::create_unprocessable;
    } else if #[cfg(feature = "sqlite")] {
    }
}
