use crate::{
    activity_pub::{ApAddress, ApContext, Outbox},
    db::Db,
    fairings::events::EventChannels,
    helper::{get_instrument_as_id_from_uuid, get_session_as_id_from_uuid},
    models::{
        actors::Actor,
        encrypted_sessions::{create_encrypted_session, EncryptedSession, NewEncryptedSession},
        olm_one_time_keys::OlmOneTimeKey,
        olm_sessions::OlmSession,
        pg::coalesced_activity::CoalescedActivity,
        //remote_encrypted_sessions::RemoteEncryptedSession,
    },
    runner, MaybeMultiple,
};
use anyhow::{anyhow, Result};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    pub instrument: MaybeMultiple<ApInstrument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    pub uuid: Option<String>,
}

impl Outbox for ApSession {
    async fn outbox(
        &self,
        conn: Db,
        events: EventChannels,
        profile: Actor,
        _raw: Value,
    ) -> Result<String, Status> {
        //handle_session(conn, events, self.clone(), profile).await
        Err(Status::NotImplemented)
    }
}

impl Default for ApSession {
    fn default() -> ApSession {
        let uuid = Uuid::new_v4().to_string();
        ApSession {
            context: Some(ApContext::default()),
            kind: ApSessionType::default(),
            id: Some(get_session_as_id_from_uuid(uuid.clone())),
            to: ApAddress::None,
            attributed_to: ApAddress::None,
            instrument: MaybeMultiple::None,
            reference: Option::None,
            uuid: Some(uuid),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApSessionType {
    #[default]
    #[serde(alias = "encrypted_session")]
    EncryptedSession,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq, Ord, PartialOrd)]
pub enum ApInstrumentType {
    #[default]
    #[serde(alias = "olm_identity_key")]
    OlmIdentityKey,
    #[serde(alias = "olm_one_time_key")]
    OlmOneTimeKey,
    #[serde(alias = "olm_session")]
    OlmSession,
    #[serde(alias = "olm_account")]
    OlmAccount,
    #[serde(alias = "vault_item")]
    VaultItem,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq, Ord, PartialOrd)]
pub struct ApInstrument {
    #[serde(rename = "type")]
    pub kind: ApInstrumentType,
    pub id: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation_of: Option<String>,
}

impl ApInstrument {
    pub fn is_olm_identity_key(&self) -> bool {
        matches!(self.kind, ApInstrumentType::OlmIdentityKey)
    }

    pub fn is_olm_one_time_key(&self) -> bool {
        matches!(self.kind, ApInstrumentType::OlmOneTimeKey)
    }

    pub fn is_olm_session(&self) -> bool {
        matches!(self.kind, ApInstrumentType::OlmSession)
    }

    pub fn is_olm_account(&self) -> bool {
        matches!(self.kind, ApInstrumentType::OlmAccount)
    }

    pub fn is_vault_item(&self) -> bool {
        matches!(self.kind, ApInstrumentType::VaultItem)
    }
}

impl Outbox for ApInstrument {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Actor,
        _raw: Value,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl TryFrom<Actor> for ApInstrument {
    type Error = anyhow::Error;

    fn try_from(actor: Actor) -> Result<Self> {
        Ok(Self {
            kind: ApInstrumentType::OlmIdentityKey,
            id: Some(format!("{}#identity-key", actor.as_id)),
            content: Some(
                actor
                    .ek_olm_identity_key
                    .ok_or(anyhow!("Actor does not have an IDK"))?,
            ),
            hash: None,
            uuid: None,
            name: None,
            url: None,
            mutation_of: None,
        })
    }
}

impl From<OlmSession> for ApInstrument {
    fn from(session: OlmSession) -> Self {
        Self {
            kind: ApInstrumentType::OlmSession,
            id: Some(get_session_as_id_from_uuid(session.uuid.clone())),
            content: Some(session.session_data),
            hash: Some(session.session_hash),
            uuid: None,
            name: None,
            url: None,
            mutation_of: None,
        }
    }
}

impl From<OlmOneTimeKey> for ApInstrument {
    fn from(otk: OlmOneTimeKey) -> Self {
        Self {
            kind: ApInstrumentType::OlmOneTimeKey,
            id: Some(get_instrument_as_id_from_uuid(otk.uuid.clone())),
            content: Some(otk.key_data),
            uuid: None,
            hash: None,
            name: None,
            url: None,
            mutation_of: None,
        }
    }
}

impl TryFrom<CoalescedActivity> for ApInstrument {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: ApInstrumentType::VaultItem,
            id: Some(get_instrument_as_id_from_uuid(
                coalesced
                    .vault_uuid
                    .clone()
                    .ok_or(anyhow!("VaultItem must have a UUID"))?,
            )),
            content: Some(
                coalesced
                    .vault_data
                    .ok_or(anyhow!("VaultItem must have content"))?,
            ),
            uuid: Some(
                coalesced
                    .vault_uuid
                    .ok_or(anyhow!("VaultItem must have a UUID"))?,
            ),
            hash: None,
            name: None,
            url: None,
            mutation_of: None,
        })
    }
}

// #[derive(Deserialize, Serialize, Clone, Debug)]
// pub struct JoinData {
//     pub session_key: ApInstrument,
//     pub identity_key: ApInstrument,
//     pub to: ApAddress,
//     pub attributed_to: ApAddress,
//     pub reference: String,
// }

// impl From<JoinData> for ApSession {
//     fn from(keys: JoinData) -> Self {
//         Self {
//             reference: Some(keys.reference),
//             to: keys.to,
//             attributed_to: keys.attributed_to,
//             instrument: MaybeMultiple::Multiple(vec![keys.identity_key, keys.session_key]),
//             ..Default::default()
//         }
//     }
// }

// impl From<EncryptedSession> for ApSession {
//     fn from(session: EncryptedSession) -> ApSession {
//         cfg_if::cfg_if! {
//             if #[cfg(feature = "pg")] {
//                 let instrument = serde_json::from_value(session.instrument).unwrap();
//             } else if #[cfg(feature = "sqlite")] {
//                 let instrument = serde_json::from_str(&session.instrument).unwrap();
//             }
//         }

//         ApSession {
//             id: Some(format!(
//                 "https://{}/session/{}",
//                 *crate::SERVER_NAME,
//                 session.uuid
//             )),
//             reference: session.reference,
//             to: ApAddress::Address(session.ap_to),
//             attributed_to: ApAddress::Address(session.attributed_to),
//             instrument,
//             uuid: Some(session.uuid),

//             ..Default::default()
//         }
//     }
// }

// type JoinedOlmSession = (ApSession, Option<OlmSession>);

// impl From<JoinedOlmSession> for ApSession {
//     fn from((ap_session, olm_session): JoinedOlmSession) -> Self {
//         let mut session = ap_session;

//         match session.instrument {
//             MaybeMultiple::Multiple(instruments) if olm_session.is_some() => {
//                 let mut instruments = instruments;
//                 instruments.push(olm_session.unwrap().into());
//                 session.instrument = MaybeMultiple::Multiple(instruments);
//             }
//             MaybeMultiple::Single(instrument) if olm_session.is_some() => {
//                 let mut instruments: Vec<ApInstrument> = vec![instrument];
//                 instruments.push(olm_session.unwrap().into());
//                 session.instrument = MaybeMultiple::Multiple(instruments);
//             }
//             _ => (),
//         }

//         session
//     }
// }

// impl From<RemoteEncryptedSession> for ApSession {
//     fn from(session: RemoteEncryptedSession) -> ApSession {
//         cfg_if::cfg_if! {
//             if #[cfg(feature = "pg")] {
//                 let instrument = serde_json::from_value(session.instrument).unwrap();
//             } else if #[cfg(feature = "sqlite")] {
//                 let instrument = serde_json::from_str(&session.instrument).unwrap();
//             }
//         }

//         ApSession {
//             id: Some(session.ap_id),
//             reference: session.reference,
//             to: ApAddress::Address(session.ap_to),
//             attributed_to: ApAddress::Address(session.attributed_to),
//             instrument,
//             ..Default::default()
//         }
//     }
// }

// async fn handle_session(
//     conn: Db,
//     channels: EventChannels,
//     session: ApSession,
//     profile: Actor,
// ) -> Result<String, Status> {
//     let encrypted_session: NewEncryptedSession = (session.clone(), profile.id).into();

//     if let Some(session) = create_encrypted_session(Some(&conn), encrypted_session.clone()).await {
//         runner::run(
//             runner::encrypted::send_kexinit_task,
//             Some(conn),
//             Some(channels),
//             vec![session.uuid.clone()],
//         )
//         .await;
//         Ok(session.uuid)
//     } else {
//         Err(Status::NoContent)
//     }
// }
