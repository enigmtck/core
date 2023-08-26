use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{
        types::signature::ApSignatureType, ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox,
    },
    db::Db,
    fairings::{events::EventChannels, faktory::FaktoryConnection},
    inbox,
    models::profiles::Profile,
    outbox, MaybeMultiple, MaybeReference,
};
use rocket::http::Status;
use rsa::pkcs8::DecodePrivateKey;
use rsa::signature::{RandomizedSigner, Signature};
use rsa::{pkcs1v15::SigningKey, RsaPrivateKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use super::signature::ApSignature;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApDeleteType {
    #[default]
    Delete,
}

impl fmt::Display for ApDeleteType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApDelete {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApDeleteType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
    pub signature: Option<ApSignature>,
    pub to: MaybeMultiple<ApAddress>,
}

impl Inbox for ApDelete {
    async fn inbox(&self, conn: Db, _faktory: FaktoryConnection) -> Result<Status, Status> {
        inbox::activity::delete(conn, self.clone()).await
    }
}

impl Outbox for ApDelete {
    async fn outbox(
        &self,
        conn: Db,
        faktory: FaktoryConnection,
        _events: EventChannels,
        profile: Profile,
    ) -> Result<String, Status> {
        outbox::activity::delete(conn, faktory, self.clone(), profile).await
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApTombstoneType {
    #[default]
    Tombstone,
}

impl fmt::Display for ApTombstoneType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApTombstone {
    #[serde(rename = "type")]
    pub kind: ApTombstoneType,
    pub id: String,
    pub atom_uri: Option<String>,
}

impl TryFrom<ApNote> for ApTombstone {
    type Error = &'static str;

    fn try_from(note: ApNote) -> Result<Self, Self::Error> {
        if let Some(id) = note.id {
            Ok(ApTombstone {
                kind: ApTombstoneType::Tombstone,
                id: id.clone(),
                atom_uri: Some(id),
            })
        } else {
            Err("ApNote must have an ID")
        }
    }
}

impl TryFrom<ApNote> for ApDelete {
    type Error = &'static str;

    fn try_from(note: ApNote) -> Result<Self, Self::Error> {
        if let (Some(id), Ok(tombstone)) = (note.id.clone(), ApTombstone::try_from(note.clone())) {
            Ok(ApDelete {
                context: Some(ApContext::default()),
                actor: note.attributed_to.clone(),
                kind: ApDeleteType::Delete,
                id: Some(format!("{id}#delete")),
                object: MaybeReference::Actual(ApObject::Tombstone(tombstone)),
                signature: None,
                to: note.to,
            })
        } else {
            Err("ApNote must have an ID")
        }
    }
}

impl ApDelete {
    // This function is based off of the description here: https://docs.joinmastodon.org/spec/security/#ld-sign
    // The content to be signed is unclear: e.g., the "verify" talks about stripping the Signature object
    // down to just created and creator, but the "signing" description doesn't talk about including that
    // information. I'm assuming it should be included since the verify will not work without it. Also, I'm
    // using the SHA256 built in to the RSA signing methods rather than handling that as a separate task.
    // That may be a mistake, but it seems like I'd be double hashing to do otherwise.

    // UPDATED: Tried to make sense of the JSON-LD documents, but this all seems unnecessarily complicated
    // I'll review some other options (like the Proof stuff that silverpill and Mitra have) to see if that's
    // more reasonable. For now, we just aren't signing these, so this will limit the ability for relayed
    // messages to be acted on.
    pub async fn sign(mut self, profile: Profile) -> Result<ApDelete, ()> {
        let document = serde_json::to_string(&self).unwrap();
        log::debug!("DOCUMENT TO BE SIGNED\n{document:#?}");

        let private_key = RsaPrivateKey::from_pkcs8_pem(&profile.private_key).unwrap();
        let signing_key = SigningKey::<Sha256>::new_with_prefix(private_key);

        let mut rng = rand::thread_rng();
        let signed_hash = signing_key.sign_with_rng(&mut rng, document.as_bytes());

        if let Some(mut signature) = self.signature {
            signature.signature_value = Some(base64::encode(signed_hash.as_bytes()));
            signature.kind = Some(ApSignatureType::RsaSignature2017);
            self.signature = Some(signature);

            Ok(self.clone())
        } else {
            Err(())
        }
    }
}
