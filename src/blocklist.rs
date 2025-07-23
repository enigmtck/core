use std::sync::Arc;

use crate::{models::instances::Instance, schema::instances};
use async_mutex::Mutex;
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;

#[derive(Debug)]
pub enum AccessControlError {
    Prohibited,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Permitted(pub bool);

impl Permitted {
    pub fn is_permitted(&self) -> bool {
        matches!(self, Permitted(true))
    }
}

#[derive(Clone)]
pub struct BlockList {
    pub blocked_servers: Arc<Mutex<Vec<String>>>,
}

impl BlockList {
    // Add this new async function specifically for Axum's pool type
    pub async fn new_axum(pool: &Pool) -> anyhow::Result<Self> {
        let conn = pool.get().await?;
        // The `??` operator fails here because the error type returned by `interact`
        // is not `Sync`, which is required by `anyhow`'s `From` trait implementation.
        // We handle it in two steps to manually convert the error.
        let query_result = conn
            .interact(move |c| {
                instances::table
                    .filter(instances::blocked.eq(true))
                    .get_results::<Instance>(c)
            })
            .await
            .map_err(|e| anyhow::anyhow!("Database interaction failed: {:?}", e))?;

        let instances = query_result?;

        log::debug!("loading {:?} blocked servers for Axum", instances.len());
        let blocked_servers: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(
            instances.iter().map(|x| x.domain_name.clone()).collect(),
        ));

        Ok(BlockList { blocked_servers })
    }

    pub fn add(&mut self, server: String) {
        log::debug!("adding {server} to BlockList");
        if let Some(mut x) = self.blocked_servers.try_lock() {
            x.push(server);
        }
    }

    pub fn is_blocked(&self, server: String) -> bool {
        log::debug!("checking {server} against BlockList");
        if let Some(x) = self.blocked_servers.try_lock() {
            x.contains(&server)
        } else {
            // fail open
            false
        }
    }
}
