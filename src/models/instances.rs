use crate::db::Db;
use crate::schema::instances;
use crate::POOL;
use diesel::prelude::*;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::instances::NewInstance;
        pub use crate::models::pg::instances::Instance;
        pub use crate::models::pg::instances::create_or_update_instance;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::instances::NewInstance;
        pub use crate::models::sqlite::instances::Instance;
        pub use crate::models::sqlite::instances::create_or_update_instance;
    }
}

impl From<String> for NewInstance {
    fn from(domain_name: String) -> Self {
        NewInstance {
            domain_name,
            json: None,
            blocked: false,
        }
    }
}

pub async fn get_instance_by_domain_name(conn: &Db, domain_name: String) -> Option<Instance> {
    conn.run(move |c| {
        instances::table
            .filter(instances::domain_name.eq(domain_name))
            .first::<Instance>(c)
    })
    .await
    .ok()
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
