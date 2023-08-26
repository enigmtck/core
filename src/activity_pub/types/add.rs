use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApAddress, ApContext, ApObject, Inbox},
    db::Db,
    fairings::faktory::FaktoryConnection,
    inbox, MaybeReference,
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApAddType {
    #[default]
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
    async fn inbox(&self, conn: Db, faktory: FaktoryConnection) -> Result<Status, Status> {
        inbox::activity::add(conn, faktory, self.clone()).await
    }
}
