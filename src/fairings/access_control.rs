use diesel::prelude::*;
use std::{collections::HashMap, sync::Arc};

use crate::{db::Db, models::instances::Instance, schema::instances, ASSIGNMENT_RE, DOMAIN_RE};

use async_mutex::Mutex;
use rocket::{
    fairing::{self, Fairing, Info, Kind},
    http::Status,
    request::{FromRequest, Outcome, Request},
    Build, Rocket,
};

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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Permitted {
    type Error = AccessControlError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let blocks = request
            .guard::<BlockList>()
            .await
            .expect("failed to retrieve BlockList");

        let signature_vec: Vec<_> = request.headers().get("signature").collect();
        let signature = signature_vec[0].to_string();
        let mut signature_map = HashMap::<String, String>::new();

        for cap in ASSIGNMENT_RE.captures_iter(&signature) {
            signature_map.insert(cap[1].to_string(), cap[2].to_string());
        }

        let key_id = signature_map
            .get("keyId")
            .expect("keyId not found in signature_map");

        let domain_name = DOMAIN_RE
            .captures(key_id)
            .expect("unable to locate domain name")[1]
            .to_string();

        if blocks.is_blocked(domain_name.clone()) {
            log::debug!("BLOCKED MESSAGE FROM {domain_name}");
            Outcome::Success(Permitted(false))
        } else {
            Outcome::Success(Permitted(true))
        }
    }
}

#[derive(Clone)]
pub struct BlockList {
    pub blocked_servers: Arc<Mutex<Vec<String>>>,
}

struct BlockListFairing;

impl BlockList {
    pub fn fairing() -> impl Fairing {
        BlockListFairing
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

#[rocket::async_trait]
impl Fairing for BlockListFairing {
    fn info(&self) -> Info {
        Info {
            name: "BlockList",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        log::debug!("igniting BlockList");
        let pool = match Db::pool(&rocket) {
            Some(pool) => pool.clone(),
            None => return Err(rocket),
        };

        if let Some(conn) = pool.get().await {
            let instances = conn
                .run(move |c| {
                    instances::table
                        .filter(instances::blocked.eq(true))
                        .get_results::<Instance>(c)
                })
                .await
                .unwrap_or(vec![]);

            log::debug!("loading {:?} blocked servers", instances.len());
            let blocked_servers: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(
                instances.iter().map(|x| x.domain_name.clone()).collect(),
            ));
            Ok(rocket.manage(BlockList { blocked_servers }))
        } else {
            Err(rocket)
        }
    }
}

#[derive(Debug)]
pub enum BlockListError {
    Failed,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for BlockList {
    type Error = BlockListError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(block_list) = request.rocket().state::<BlockList>() {
            Outcome::Success(block_list.clone())
        } else {
            Outcome::Error((Status::BadRequest, BlockListError::Failed))
        }
    }
}
