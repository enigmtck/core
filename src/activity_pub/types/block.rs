use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApAddress, ApContext, Inbox, Outbox},
    db::Db,
    fairings::{events::EventChannels, faktory::FaktoryConnection},
    models::profiles::Profile,
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApBlockType {
    #[default]
    Block,
}

impl fmt::Display for ApBlockType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApBlock {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApBlockType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub object: String,
}

impl Inbox for ApBlock {
    async fn inbox(
        &self,
        _conn: Db,
        _channels: EventChannels,
        _faktory: FaktoryConnection,
        raw: Value,
    ) -> Result<Status, Status> {
        log::warn!("BLOCK ACTIVITY NOT YET IMPLEMENTED");
        log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
        Err(Status::NoContent)
    }
}

impl Outbox for ApBlock {
    async fn outbox(
        &self,
        _conn: Db,
        _faktory: FaktoryConnection,
        _events: EventChannels,
        _profile: Profile,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
}
