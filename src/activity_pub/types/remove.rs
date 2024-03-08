use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApAddress, ApContext, ApObject, Inbox, Outbox},
    db::Db,
    fairings::{events::EventChannels, faktory::FaktoryConnection},
    models::profiles::Profile,
    MaybeReference,
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApRemoveType {
    #[default]
    Remove,
}

impl fmt::Display for ApRemoveType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApRemove {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApRemoveType,
    pub actor: ApAddress,
    pub target: Option<String>,
    pub object: MaybeReference<ApObject>,
}

impl Inbox for ApRemove {
    async fn inbox(
        &self,
        _conn: Db,
        _channels: EventChannels,
        _faktory: FaktoryConnection,
        raw: Value,
    ) -> Result<Status, Status> {
        log::warn!("REMOVE ACTIVITY NOT YET IMPLEMENTED");
        log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
        Err(Status::NoContent)
    }
}

impl Outbox for ApRemove {
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
