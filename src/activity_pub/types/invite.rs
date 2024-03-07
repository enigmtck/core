use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApAddress, ApContext, ApInstruments, ApObject, ApSession, Inbox, Outbox},
    db::Db,
    fairings::{events::EventChannels, faktory::FaktoryConnection},
    models::{
        profiles::{get_profile_by_ap_id, Profile},
        remote_encrypted_sessions::create_remote_encrypted_session,
    },
    to_faktory, MaybeMultiple, MaybeReference,
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    async fn inbox(
        &self,
        conn: Db,
        faktory: FaktoryConnection,
        raw: Value,
    ) -> Result<Status, Status> {
        log::debug!("PROCESSING INVITE\n{self:#?}");

        if let Ok(ApAddress::Address(to)) = self.to.clone().single() {
            if let Some(profile) = get_profile_by_ap_id(Some(&conn), to).await {
                if let Some(session) =
                    create_remote_encrypted_session(&conn, (self.clone(), profile.id).into()).await
                {
                    to_faktory(faktory, "provide_one_time_key", vec![session.ap_id])
                } else {
                    log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                    Err(Status::NoContent)
                }
            } else {
                log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                Err(Status::NoContent)
            }
        } else {
            log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
            Err(Status::NoContent)
        }
    }
}

impl Outbox for ApInvite {
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
