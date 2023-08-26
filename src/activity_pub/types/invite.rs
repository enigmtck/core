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
pub enum ApInviteType {
    #[default]
    Invite,
}

impl fmt::Display for ApInviteType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApInvite {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApInviteType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub to: MaybeMultiple<ApAddress>,
    pub object: MaybeReference<ApObject>,
}

impl Inbox for ApInvite {
    async fn inbox(&self, conn: Db, faktory: FaktoryConnection) -> Result<Status, Status> {
        inbox::activity::invite(conn, faktory, self.clone()).await
    }
}

impl TryFrom<ApSession> for ApInvite {
    type Error = &'static str;
    fn try_from(session: ApSession) -> Result<Self, Self::Error> {
        if let ApInstruments::Single(_) = session.instrument {
            Ok(ApInvite {
                context: Some(ApContext::default()),
                kind: ApInviteType::default(),
                actor: session.attributed_to.clone(),
                id: session.id.clone().map(|id| format!("{id}#invite")),
                to: MaybeMultiple::Single(session.to.clone()),
                object: ApObject::Session(session).into(),
            })
        } else {
            Err("MULTIPLE INSTRUMENTS DETECTED - THIS LOOKS LIKE A JOIN")
        }
    }
}
