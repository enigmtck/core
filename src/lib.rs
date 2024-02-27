#![allow(async_fn_in_trait)]
extern crate log;

#[macro_use]
extern crate rocket;

extern crate diesel;

use activity_pub::{ApActivity, ApObject};
use anyhow::Result;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use dotenvy::dotenv;
use fairings::faktory::assign_to_faktory;
use fairings::faktory::FaktoryConnection;
use lazy_static::lazy_static;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;

pub mod activity_pub;
pub mod admin;
pub mod api;
pub mod db;
pub mod fairings;
pub mod helper;
pub mod inbox;
pub mod models;
pub mod outbox;
pub mod routes;
pub mod runner;
pub mod schema;
pub mod server;
pub mod signing;
pub mod webfinger;

lazy_static! {
    pub static ref POOL: Pool<ConnectionManager<PgConnection>> = {
        dotenv().ok();
        Pool::new(ConnectionManager::<PgConnection>::new(
            env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        ))
        .expect("failed to create db pool")
    };
    pub static ref SERVER_URL: String = {
        dotenv().ok();
        env::var("SERVER_URL").expect("SERVER_URL must be set")
    };
    pub static ref SERVER_NAME: String = {
        dotenv().ok();
        env::var("SERVER_NAME").expect("SERVER_NAME must be set")
    };
    pub static ref SYSTEM_USER: String = {
        dotenv().ok();
        env::var("SYSTEM_USER").expect("SYSTEM_USER must be set")
    };
    pub static ref MEDIA_DIR: String = {
        dotenv().ok();
        env::var("MEDIA_DIR").expect("MEDIA_DIR must be set")
    };
    pub static ref FAKTORY_URL: String = {
        dotenv().ok();
        env::var("FAKTORY_URL").expect("FAKTORY_URL must be set")
    };
    pub static ref AMQP_URL: String = {
        dotenv().ok();
        env::var("AMQP_URL").expect("AMQP_URL must be set")
    };
    pub static ref AMQP_CONSUMER_TAG: String = {
        dotenv().ok();
        env::var("AMQP_CONSUMER_TAG").expect("AMQP_CONSUMER_TAG must be set")
    };
    pub static ref AMQP_QUEUE: String = {
        dotenv().ok();
        env::var("AMQP_QUEUE").expect("AMQP_QUEUE must be set")
    };
    pub static ref REGISTRATION_ENABLED: bool = {
        dotenv().ok();
        env::var("REGISTRATION_ENABLED")
            .expect("REGISTRATION_ENABLED must be set")
            .parse()
            .expect("REGISTRATION_ENABLED must be true or false")
    };
    pub static ref REGISTRATION_APPROVAL_REQUIRED: bool = {
        dotenv().ok();
        env::var("REGISTRATION_APPROVAL_REQUIRED")
            .expect("REGISTRATION_APPROVAL_REQUIRED must be set")
            .parse()
            .expect("REGISTRATION_APPROVAL_REQUIRED must be true or false")
    };
    pub static ref REGISTRATION_MESSAGE: String = {
        dotenv().ok();
        env::var("REGISTRATION_MESSAGE").expect("REGISTRATION_MESSAGE must be set")
    };
    pub static ref INSTANCE_CONTACT: String = {
        dotenv().ok();
        env::var("INSTANCE_CONTACT").expect("INSTANCE_CONTACT must be set")
    };
    pub static ref INSTANCE_TITLE: String = {
        dotenv().ok();
        env::var("INSTANCE_TITLE").expect("INSTANCE_TITLE must be set")
    };
    pub static ref INSTANCE_VERSION: String = {
        dotenv().ok();
        env::var("INSTANCE_VERSION").expect("INSTANCE_VERSION must be set")
    };
    pub static ref INSTANCE_SOURCE_URL: String = {
        dotenv().ok();
        env::var("INSTANCE_SOURCE_URL").expect("INSTANCE_SOURCE_URL must be set")
    };
    pub static ref INSTANCE_DESCRIPTION: String = {
        dotenv().ok();
        env::var("INSTANCE_DESCRIPTION").expect("INSTANCE_DESCRIPTION must be set")
    };
}

// This was an idea I had to return an un-awaited Future from the model calls so that I could use
// non-async calls in runner. But it's truly a pain in the ass to deal with the 'static lifetime requirements
// of Rocket's async conn. I couldn't make it work properly with all the other requirements.

// pub enum MaybeFuture<'a, T: Clone> {
//     Future(Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>),
//     Resolved(Result<T>),
// }

// impl<'a, T: Clone> From<Pin<Box<dyn Future<Output = Result<T>> + Send>>> for MaybeFuture<'a, T> {
//     fn from(future: Pin<Box<dyn Future<Output = Result<T>> + Send>>) -> Self {
//         MaybeFuture::Future(future)
//     }
// }

// impl<'a, T: Clone> From<Result<T>> for MaybeFuture<'a, T> {
//     fn from(result: Result<T>) -> Self {
//         MaybeFuture::Resolved(result)
//     }
// }

// impl<'a, T: Clone> MaybeFuture<'a, T> {
//     pub fn resolved(self) -> Result<T> {
//         match self {
//             MaybeFuture::Resolved(result) => result,
//             _ => Err(anyhow::Error::msg("MaybeFuture is not Resolved")),
//         }
//     }

//     pub async fn future(self) -> Result<T> {
//         match self {
//             MaybeFuture::Future(future) => future.await,
//             _ => Err(anyhow::Error::msg("MaybeFuture is not Future")),
//         }
//     }
// }

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(untagged)]
pub enum MaybeMultiple<T> {
    Single(T),
    Multiple(Vec<T>),
    #[default]
    None,
}

impl From<String> for MaybeMultiple<String> {
    fn from(data: String) -> Self {
        MaybeMultiple::Single(data)
    }
}

impl<T> From<Vec<T>> for MaybeMultiple<T> {
    fn from(data: Vec<T>) -> Self {
        MaybeMultiple::Multiple(data)
    }
}

impl<T: Clone> MaybeMultiple<T> {
    pub fn single(&self) -> Option<T> {
        match self {
            MaybeMultiple::Multiple(s) => {
                if s.len() == 1 {
                    Some(s[0].clone())
                } else {
                    None
                }
            }
            MaybeMultiple::Single(s) => Some(s.clone()),
            MaybeMultiple::None => None,
        }
    }

    pub fn multiple(&self) -> Vec<T> {
        match self {
            MaybeMultiple::Multiple(data) => data.clone(),
            MaybeMultiple::Single(data) => {
                vec![data.clone()]
            }
            MaybeMultiple::None => vec![],
        }
    }

    pub fn extend(mut self, mut additional: Vec<T>) -> Self {
        match self {
            MaybeMultiple::Multiple(ref mut data) => {
                data.append(&mut additional);
                data.clone().into()
            }
            MaybeMultiple::Single(data) => {
                additional.push(data.clone());
                additional.clone().into()
            }
            MaybeMultiple::None => additional.clone().into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Identifier {
    id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(untagged)]
pub enum MaybeReference<T> {
    Reference(String),
    Actual(T),
    Identifier(Identifier),
    #[default]
    None,
}

impl<T> MaybeReference<T> {
    pub fn reference(&self) -> Option<String> {
        match self {
            MaybeReference::Reference(reference) => Some(reference.clone()),
            MaybeReference::Identifier(identifier) => Some(identifier.id.clone()),
            _ => None,
        }
    }

    pub fn actual(&self) -> Option<&T> {
        match self {
            MaybeReference::Actual(actual) => Some(actual),
            _ => None,
        }
    }
}

impl<T> fmt::Display for MaybeReference<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MaybeReference::Reference(reference) => f.write_str(reference),
            MaybeReference::Identifier(identifier) => f.write_str(&identifier.id),
            _ => f.write_str("UNDEFINED"),
        }
    }
}

impl From<ApObject> for MaybeReference<ApObject> {
    fn from(object: ApObject) -> Self {
        MaybeReference::Actual(object)
    }
}

impl From<ApActivity> for MaybeReference<ApActivity> {
    fn from(activity: ApActivity) -> Self {
        MaybeReference::Actual(activity)
    }
}

impl From<String> for MaybeReference<String> {
    fn from(reference: String) -> Self {
        MaybeReference::Reference(reference)
    }
}

pub fn to_faktory(
    faktory: FaktoryConnection,
    operation: &str,
    id: String,
) -> Result<Status, Status> {
    match assign_to_faktory(faktory, String::from(operation), vec![id]) {
        Ok(_) => Ok(Status::Accepted),
        Err(e) => {
            log::error!("FAILED TO ASSIGN TO FAKTORY\n{e:#?}");
            Err(Status::NoContent)
        }
    }
}
