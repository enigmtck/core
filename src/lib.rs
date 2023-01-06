extern crate log;
extern crate rocket;

#[macro_use]
extern crate diesel;

use dotenvy::dotenv;
use lazy_static::lazy_static;
use std::env;

use faktory::{Job, Producer};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use rocket::http::RawStr;
use rocket::request::{FromParam, FromRequest, Request, Outcome};
use rocket::serde::json::Json;
use rocket::serde::json::Error;
use rocket::http::{Status, Header};
use rocket::fairing::{Fairing, Info, Kind, self};
use rocket::{Rocket, Build, Response};

pub mod activity_pub;
pub mod admin;
pub mod db;
pub mod helper;
pub mod inbox;
pub mod models;
pub mod outbox;
pub mod schema;
pub mod signing;
pub mod webfinger;

lazy_static! {
    pub static ref DATABASE_URL: String = {
        dotenv().ok();
        env::var("DATABASE_URL").expect("DATABASE_URL must be set")
    };
    pub static ref SERVER_URL: String = {
        dotenv().ok();
        env::var("SERVER_URL").expect("SERVER_URL must be set")
    };
    pub static ref SERVER_NAME: String = {
        dotenv().ok();
        env::var("SERVER_NAME").expect("SERVER_NAME must be set")
    };
    pub static ref FAKTORY_URL: String = {
        dotenv().ok();
        env::var("FAKTORY_URL").expect("FAKTORY_URL must be set")
    };
}

#[derive(Clone)]
pub struct FaktoryConnection {
    pub producer: Arc<Mutex<Producer<TcpStream>>>,
}

impl FaktoryConnection {
    pub fn fairing() -> impl Fairing {
        FaktoryConnectionFairing
    }
}

struct FaktoryConnectionFairing;

#[rocket::async_trait]
impl Fairing for FaktoryConnectionFairing {
    fn info(&self) -> Info {
        Info {
            name: "Faktory Connection",
            kind: Kind::Ignite
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        Ok(rocket.manage(FaktoryConnection {
            producer: Arc::new(
                Mutex::new(
                    Producer::connect(
                        Some("tcp://:password@localhost:7419")).unwrap()
                )
            )
        }))
    }
}

#[derive(Debug)]
pub enum FaktoryConnectionError {
    Failed
}

#[rocket::async_trait]
impl <'r> FromRequest<'r> for FaktoryConnection {
    type Error = FaktoryConnectionError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(faktory) = request.rocket().state::<FaktoryConnection>() {
            Outcome::Success(faktory.clone())
        } else {
            Outcome::Failure((Status::BadRequest, FaktoryConnectionError::Failed))
        }
    }
}
