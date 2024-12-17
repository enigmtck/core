#![feature(async_closure)]
#![allow(async_fn_in_trait)]
extern crate diesel;
extern crate log;
#[macro_use]
extern crate rocket;

use activity_pub::ApCollection;
use activity_pub::{ApActivity, ApObject};
use anyhow::anyhow;
use anyhow::Result;
use db::Pool;
use diesel::r2d2::ConnectionManager;
use dotenvy::dotenv;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::env;
use std::fmt;

pub mod activity_pub;
pub mod admin;
pub mod db;
pub mod fairings;
pub mod helper;
pub mod models;
pub mod routes;
pub mod runner;

#[cfg(all(feature = "pg", feature = "sqlite"))]
compile_error!("Features 'pg' and 'sqlite' cannot be enabled at the same time.");

#[cfg(feature = "pg")]
#[path = "schema-pg.rs"]
pub mod schema;

#[cfg(feature = "sqlite")]
#[path = "schema-sqlite.rs"]
pub mod schema;

pub mod server;
pub mod signing;
pub mod webfinger;

lazy_static! {
    pub static ref IMAGE_MEDIA_RE: Regex =
        Regex::new(r#"^image\/([a-z]+)$"#).expect("invalid image media regex");
    pub static ref ANCHOR_RE: Regex =
        Regex::new(r#"<a href="(.+?)".*?>"#).expect("invalid anchor regex");
    pub static ref WEBFINGER_RE: Regex =
        Regex::new(r#"@(.+?)@(.+)"#).expect("invalid webfinger regex");
    pub static ref WEBFINGER_ACCT_RE: Regex =
        Regex::new(r#"acct:(.+?)@(.+)"#).expect("invalid webfinger acct regex");
    pub static ref LOCAL_RE: Regex =
        Regex::new(&format!(r#"\w+?://{}/(.+)"#, *SERVER_NAME)).expect("invalid local regex");
    pub static ref DOMAIN_RE: Regex =
        Regex::new(&format!(r#"https://([\w\.-]+)/?(.*)"#)).expect("invalid domain regex");
    pub static ref LOCAL_URL_RE: Regex = Regex::new(&format!(
        r#"^{}/(user|notes|session|collections|activities|objects|instruments)/(.+)$"#,
        *SERVER_URL
    ))
    .expect("invalid local url regex");
    pub static ref LOCAL_USER_KEY_ID_RE: Regex =
        Regex::new(&format!(r#"(\w+://{}/user/(.+?))#(.+)"#, *SERVER_NAME))
            .expect("invalid local user key id regex");
    pub static ref ASSIGNMENT_RE: Regex =
        Regex::new(r#"(\w+)="(.+?)""#).expect("invalid assignment regex");
    pub static ref POOL: Pool = {
        dotenv().ok();
        cfg_if::cfg_if! {
            if #[cfg(feature = "pg")] {
                Pool::new(ConnectionManager::<diesel::PgConnection>::new(
                    env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
                ))
                    .expect("failed to create db pool")
            } else if #[cfg(feature = "sqlite")] {
                Pool::new(ConnectionManager::<diesel::SqliteConnection>::new(
                    env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
                ))
                    .expect("failed to create db pool")
            }
        }
    };
    pub static ref DEFAULT_AVATAR: String = {
        dotenv().ok();
        env::var("DEFAULT_AVATAR").expect("DEFAULT_AVATAR must be set")
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

    // SIGNING_OVERRIDE turns off signature checking so that I can test the API using curl
    pub static ref SIGNING_OVERRIDE: bool = {
        dotenv().ok();
        env::var("SIGNING_OVERRIDE")
            .ok()
            .and_then(|x| x.parse().ok())
            .unwrap_or(false)
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

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
#[serde(untagged)]
pub enum MaybeMultiple<T> {
    Single(T),
    Multiple(Vec<T>),
    #[default]
    None,
}

impl<T: PartialEq> MaybeMultiple<T> {
    fn is_none(&self) -> bool {
        *self == MaybeMultiple::None
    }
}

impl<T> From<Option<Vec<T>>> for MaybeMultiple<T> {
    fn from(data: Option<Vec<T>>) -> Self {
        match data {
            Some(x) => MaybeMultiple::Multiple(x),
            None => MaybeMultiple::None,
        }
    }
}

impl<T: DeserializeOwned> From<Option<Value>> for MaybeMultiple<T> {
    fn from(data: Option<Value>) -> Self {
        match data {
            Some(value) => value.into(),
            None => MaybeMultiple::None,
        }
    }
}

impl<T: DeserializeOwned> From<Value> for MaybeMultiple<T> {
    fn from(data: Value) -> Self {
        // First, try to convert to Vec<T>
        if let Ok(vec_result) = serde_json::from_value::<Vec<T>>(data.clone()) {
            MaybeMultiple::Multiple(vec_result)
        }
        // If Vec conversion fails, try single T
        else if let Ok(single_result) = serde_json::from_value::<T>(data) {
            MaybeMultiple::Single(single_result)
        }
        // If both conversions fail, return None
        else {
            MaybeMultiple::None
        }
    }
}

impl<T: Serialize> From<&MaybeMultiple<T>> for Option<Value> {
    fn from(object: &MaybeMultiple<T>) -> Self {
        match object {
            MaybeMultiple::None => None,
            _ => Some(json!(object)),
        }
    }
}

impl<T: Serialize> From<&MaybeMultiple<T>> for Value {
    fn from(object: &MaybeMultiple<T>) -> Self {
        json!(object)
    }
}

impl From<ApObject> for MaybeMultiple<ApObject> {
    fn from(data: ApObject) -> Self {
        MaybeMultiple::Single(data)
    }
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
    pub fn single(&self) -> Result<T> {
        match self {
            MaybeMultiple::Multiple(s) => {
                if s.len() == 1 {
                    Ok(s[0].clone())
                } else {
                    Err(anyhow!("MaybeMultiple is Multiple"))
                }
            }
            MaybeMultiple::Single(s) => Ok(s.clone()),
            MaybeMultiple::None => Err(anyhow!("MaybeMultiple is None")),
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

    pub fn actual(self) -> Option<T> {
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

impl From<String> for MaybeReference<ApObject> {
    fn from(reference: String) -> Self {
        MaybeReference::Reference(reference)
    }
}

impl From<String> for MaybeReference<Box<ApCollection>> {
    fn from(reference: String) -> Self {
        MaybeReference::Reference(reference)
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

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct OrdValue(Value);

impl PartialOrd for OrdValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrdValue {
    fn cmp(&self, other: &Self) -> Ordering {
        let a_str = serde_json::to_string(&self.0).unwrap();
        let b_str = serde_json::to_string(&other.0).unwrap();
        a_str.cmp(&b_str)
    }
}
