use crate::{
    activity_pub::{ApAddress, ApContext},
    helper::{get_instrument_as_id_from_uuid, get_session_as_id_from_uuid},
    models::{
        actors::Actor, coalesced_activity::CoalescedActivity, olm_one_time_keys::OlmOneTimeKey,
        olm_sessions::OlmSession,
    },
    MaybeMultiple,
};
use anyhow::{anyhow, Result};
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
    pub instrument: MaybeMultiple<ApInstrument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    pub uuid: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity: Option<String>,
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

impl ApInstrument {
    pub fn from_actor_olm_account(actor: Actor) -> Self {
        ApInstrument {
            kind: ApInstrumentType::OlmAccount,
            id: Some(format!("{}#olm-account", actor.as_id)),
            content: actor.ek_olm_pickled_account,
            hash: actor.ek_olm_pickled_account_hash,
            uuid: None,
            name: None,
            url: None,
            mutation_of: None,
            conversation: None,
            activity: None,
        }
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
            conversation: None,
            activity: None,
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
            conversation: None,
            activity: None,
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
            conversation: None,
            activity: None,
        }
    }
}

impl TryFrom<CoalescedActivity> for Vec<ApInstrument> {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        let mut instruments: Vec<ApInstrument> = vec![];

        if coalesced.vault_data.is_some() {
            instruments.push(ApInstrument {
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
                conversation: None,
                activity: None,
            });
        }

        if coalesced.olm_data.is_some() {
            instruments.push(ApInstrument {
                kind: ApInstrumentType::OlmSession,
                id: Some(get_instrument_as_id_from_uuid(
                    coalesced
                        .olm_uuid
                        .clone()
                        .ok_or(anyhow!("OlmSession must have a UUID"))?,
                )),
                content: Some(
                    coalesced
                        .olm_data
                        .ok_or(anyhow!("OlmSession must have Data"))?,
                ),
                uuid: Some(
                    coalesced
                        .olm_uuid
                        .ok_or(anyhow!("OlmSession must have a UUID"))?,
                ),
                hash: None,
                name: None,
                url: None,
                mutation_of: None,
                conversation: None,
                activity: None,
            });
        }

        Ok(instruments)
    }
}
