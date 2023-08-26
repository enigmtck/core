use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApAddress, ApContext, ApInstruments, ApObject, ApSession, Inbox},
    db::Db,
    fairings::faktory::FaktoryConnection,
    inbox, MaybeMultiple, MaybeReference,
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApJoinType {
    #[default]
    Join,
}

impl fmt::Display for ApJoinType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApJoin {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApJoinType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub to: MaybeMultiple<ApAddress>,
    pub object: MaybeReference<ApObject>,
}

impl Inbox for ApJoin {
    async fn inbox(&self, conn: Db, faktory: FaktoryConnection) -> Result<Status, Status> {
        inbox::activity::join(conn, faktory, self.clone()).await
    }
}

impl TryFrom<ApSession> for ApJoin {
    type Error = &'static str;
    fn try_from(session: ApSession) -> Result<Self, Self::Error> {
        if let ApInstruments::Multiple(_) = session.instrument {
            Ok(ApJoin {
                context: Some(ApContext::default()),
                kind: ApJoinType::default(),
                actor: session.attributed_to.clone(),
                id: session.id.clone().map(|id| format!("{id}#join")),
                to: MaybeMultiple::Single(session.to.clone()),
                object: ApObject::Session(session).into(),
            })
        } else {
            Err("SINGLE INSTRUMENT DETECTED - THIS LOOKS LIKE AN INVITE")
        }
    }
}
