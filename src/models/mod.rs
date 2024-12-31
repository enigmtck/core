use actors::Actor;
use objects::Object;

pub mod activities;
pub mod actors;
pub mod cache;
pub mod coalesced_activity;
pub mod followers;
pub mod instances;
pub mod leaders;
pub mod notifications;
pub mod objects;
pub mod olm_one_time_keys;
pub mod olm_sessions;
pub mod profiles;
pub mod unprocessable;
pub mod vault;
use serde_json::Value;

#[derive(Clone, Debug)]
pub enum Tombstone {
    Actor(Actor),
    Object(Object),
}

pub fn parameter_generator() -> impl FnMut() -> String {
    let mut counter = 1;
    move || {
        let param = format!("${}", counter);
        counter += 1;
        param
    }
}

pub fn from_serde<T: serde::de::DeserializeOwned>(object: Value) -> Option<T> {
    serde_json::from_value(object).ok()
}

pub fn from_serde_option<T: serde::de::DeserializeOwned>(object: Option<Value>) -> Option<T> {
    object.and_then(|o| serde_json::from_value(o).ok())
}

pub struct OffsetPaging {
    pub page: u32,
    pub limit: u32,
}
