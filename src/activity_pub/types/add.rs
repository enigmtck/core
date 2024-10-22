use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApAddress, ApContext, ApObject, Inbox, Outbox},
    db::Db,
    fairings::events::EventChannels,
    models::actors::Actor,
    MaybeReference,
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApAddType {
    #[default]
    #[serde(alias = "add")]
    Add,
}

impl fmt::Display for ApAddType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApAdd {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApAddType,
    pub actor: ApAddress,
    pub target: Option<String>,
    pub object: MaybeReference<ApObject>,
}

impl Inbox for ApAdd {
    async fn inbox(
        &self,
        _conn: Db,
        _channels: EventChannels,
        raw: Value,
    ) -> Result<Status, Status> {
        log::warn!("ADD ACTIVITY NOT YET IMPLEMENTED");
        log::error!("{raw}");
        Err(Status::NoContent)
    }
}

impl Outbox for ApAdd {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Actor,
        raw: Value,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
}
