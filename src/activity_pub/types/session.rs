use crate::{
    activity_pub::{ApAddress, ApContext, Outbox},
    db::Db,
    fairings::{events::EventChannels, faktory::FaktoryConnection},
    models::{
        encrypted_sessions::EncryptedSession, olm_sessions::OlmSession, profiles::Profile,
        remote_encrypted_sessions::RemoteEncryptedSession,
    },
    outbox,
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApSession {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApSessionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub to: ApAddress,
    pub attributed_to: ApAddress,
    pub instrument: ApInstruments,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    pub uuid: Option<String>,
}

impl Outbox for ApSession {
    async fn outbox(
        &self,
        conn: Db,
        faktory: FaktoryConnection,
        _events: EventChannels,
        profile: Profile,
    ) -> Result<String, Status> {
        outbox::object::session(conn, faktory, self.clone(), profile).await
    }
}

impl Default for ApSession {
    fn default() -> ApSession {
        let uuid = Uuid::new_v4();
        ApSession {
            context: Option::from(ApContext::default()),
            kind: ApSessionType::default(),
            id: Option::from(format!("https://{}/session/{}", *crate::SERVER_NAME, uuid)),
            to: ApAddress::None,
            attributed_to: ApAddress::None,
            instrument: ApInstruments::default(),
            reference: Option::None,
            uuid: Some(uuid.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApSessionType {
    #[default]
    EncryptedSession,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApInstrumentType {
    #[default]
    IdentityKey,
    SessionKey,
    OlmSession,
    Service,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ApInstrument {
    #[serde(rename = "type")]
    pub kind: ApInstrumentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl Outbox for ApInstrument {
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

impl From<OlmSession> for ApInstrument {
    fn from(session: OlmSession) -> Self {
        ApInstrument {
            kind: ApInstrumentType::OlmSession,
            content: Some(session.session_data),
            hash: Some(session.session_hash),
            uuid: Some(session.uuid),
            name: None,
            url: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(untagged)]
pub enum ApInstruments {
    Multiple(Vec<ApInstrument>),
    Single(ApInstrument),
    #[default]
    Unknown,
}

#[derive(Deserialize, Clone, Debug)]
pub struct JoinData {
    pub one_time_key: String,
    pub identity_key: String,
    pub to: String,
    pub attributed_to: String,
    pub reference: String,
}

impl From<JoinData> for ApSession {
    fn from(keys: JoinData) -> ApSession {
        ApSession {
            reference: Option::from(keys.reference),
            to: ApAddress::Address(keys.to),
            attributed_to: ApAddress::Address(keys.attributed_to),
            instrument: ApInstruments::Multiple(vec![
                ApInstrument {
                    kind: ApInstrumentType::IdentityKey,
                    content: Some(keys.identity_key),
                    ..Default::default()
                },
                ApInstrument {
                    kind: ApInstrumentType::SessionKey,
                    content: Some(keys.one_time_key),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        }
    }
}

impl From<EncryptedSession> for ApSession {
    fn from(session: EncryptedSession) -> ApSession {
        ApSession {
            id: Option::from(format!(
                "https://{}/session/{}",
                *crate::SERVER_NAME,
                session.uuid
            )),
            reference: session.reference,
            to: ApAddress::Address(session.ap_to),
            attributed_to: ApAddress::Address(session.attributed_to),
            instrument: serde_json::from_value(session.instrument).unwrap(),
            uuid: Some(session.uuid),

            ..Default::default()
        }
    }
}

type JoinedOlmSession = (ApSession, Option<OlmSession>);

impl From<JoinedOlmSession> for ApSession {
    fn from((ap_session, olm_session): JoinedOlmSession) -> Self {
        let mut session = ap_session;

        match session.instrument {
            ApInstruments::Multiple(instruments) if olm_session.is_some() => {
                let mut instruments = instruments;
                instruments.push(olm_session.unwrap().into());
                session.instrument = ApInstruments::Multiple(instruments);
            }
            ApInstruments::Single(instrument) if olm_session.is_some() => {
                let mut instruments: Vec<ApInstrument> = vec![instrument];
                instruments.push(olm_session.unwrap().into());
                session.instrument = ApInstruments::Multiple(instruments);
            }
            _ => (),
        }

        session
    }
}

impl From<RemoteEncryptedSession> for ApSession {
    fn from(session: RemoteEncryptedSession) -> ApSession {
        let instrument: ApInstruments = serde_json::from_value(session.instrument).unwrap();

        ApSession {
            id: Option::from(session.ap_id),
            reference: session.reference,
            to: ApAddress::Address(session.ap_to),
            attributed_to: ApAddress::Address(session.attributed_to),
            instrument,
            ..Default::default()
        }
    }
}
