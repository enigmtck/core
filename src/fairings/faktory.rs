use faktory::{Job, Producer};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use rocket::fairing::{self, Fairing, Info, Kind};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::{Build, Rocket};

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
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        log::debug!("igniting FaktoryConnection");
        Ok(rocket.manage(FaktoryConnection {
            producer: Arc::new(Mutex::new(
                Producer::connect(Some(&*crate::FAKTORY_URL)).unwrap(),
            )),
        }))
    }
}

#[derive(Debug)]
pub enum FaktoryConnectionError {
    Failed,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for FaktoryConnection {
    type Error = FaktoryConnectionError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(faktory) = request.rocket().state::<FaktoryConnection>() {
            Outcome::Success(faktory.clone())
        } else {
            Outcome::Error((Status::BadRequest, FaktoryConnectionError::Failed))
        }
    }
}

pub fn assign_to_faktory(
    faktory: FaktoryConnection,
    job_name: String,
    job_args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    match faktory.producer.try_lock() {
        Ok(mut x) => x
            .enqueue(Job::new(job_name, job_args))
            .map_err(|e| e.into()),
        Err(e) => Err(Box::from(e.to_string())),
    }
}
