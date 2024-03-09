use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApAddress, ApContext, ApInstruments, ApObject, ApSession, Inbox, Outbox},
    db::Db,
    fairings::events::EventChannels,
    models::{
        profiles::{get_profile_by_ap_id, Profile},
        remote_encrypted_sessions::create_remote_encrypted_session,
    },
    runner, MaybeMultiple, MaybeReference,
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    async fn inbox(&self, conn: Db, channels: EventChannels, raw: Value) -> Result<Status, Status> {
        log::debug!("PROCESSING JOIN ACTIVITY\n{self:#?}");

        if let Ok(ApAddress::Address(to)) = self.to.clone().single() {
            if let Some(profile) = get_profile_by_ap_id(Some(&conn), to.clone()).await {
                if create_remote_encrypted_session(&conn, (self.clone(), profile.id).into())
                    .await
                    .is_some()
                {
                    if let MaybeReference::Actual(ApObject::Session(session)) = self.object.clone()
                    {
                        if let Some(ap_id) = session.id {
                            log::debug!("ASSIGNING JOIN ACTIVITY TO FAKTORY");

                            runner::run(
                                runner::encrypted::process_join_task,
                                Some(conn),
                                Some(channels),
                                vec![ap_id],
                            )
                            .await;
                            Ok(Status::Accepted)
                        } else {
                            log::error!("MISSING ID");
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

impl Outbox for ApJoin {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Profile,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
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
