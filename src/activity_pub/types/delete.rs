use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{
        types::signature::ApSignatureType, ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox,
    },
    db::Db,
    fairings::events::EventChannels,
    helper::{get_activity_ap_id_from_uuid, get_ap_id_from_username},
    models::{
        activities::{create_activity, ActivityType, NewActivity},
        notes::{get_notey, NoteLike},
        profiles::Profile,
        remote_actors::delete_remote_actor_by_ap_id,
        remote_notes::{delete_remote_note_by_ap_id, get_remote_note_by_ap_id},
        timeline::delete_timeline_item_by_ap_id,
    },
    runner, MaybeMultiple, MaybeReference,
};
use rocket::http::Status;
// use rsa::pkcs8::DecodePrivateKey;
// use rsa::signature::{RandomizedSigner, Signature};
// use rsa::{pkcs1v15::SigningKey, RsaPrivateKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
//use sha2::Sha256;

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
    pub cc: Option<MaybeMultiple<ApAddress>>,
}

impl Inbox for Box<ApDelete> {
    async fn inbox(
        &self,
        conn: Db,
        _channels: EventChannels,
        raw: Value,
    ) -> Result<Status, Status> {
        async fn delete_actor(conn: &Db, ap_id: String) -> Result<Status, Status> {
            if delete_remote_actor_by_ap_id(conn, ap_id).await {
                log::debug!("REMOTE ACTOR RECORD DELETED");
                Ok(Status::Accepted)
            } else {
                Err(Status::NoContent)
            }
        }

        async fn delete_note(conn: &Db, ap_id: String) -> Result<Status, Status> {
            if delete_remote_note_by_ap_id(conn, ap_id).await {
                log::debug!("REMOTE NOTE RECORD DELETED");
                Ok(Status::Accepted)
            } else {
                Err(Status::NoContent)
            }
        }

        async fn delete_timeline(conn: &Db, ap_id: String) -> Result<Status, Status> {
            if delete_timeline_item_by_ap_id(conn.into(), ap_id)
                .await
                .is_ok()
            {
                log::debug!("TIMELINE RECORD DELETED");
                Ok(Status::Accepted)
            } else {
                Err(Status::NoContent)
            }
        }

        match self.object.clone() {
            MaybeReference::Actual(actual) => match actual {
                ApObject::Tombstone(tombstone) => {
                    let remote_note = get_remote_note_by_ap_id(Some(&conn), tombstone.id.clone())
                        .await
                        .ok_or(Status::NotFound)?;
                    if remote_note.attributed_to == self.actor.clone().to_string() {
                        if delete_note(&conn, tombstone.id.clone()).await.is_ok() {
                            delete_timeline(&conn, tombstone.id).await
                        } else {
                            Err(Status::new(520))
                        }
                    } else {
                        Err(Status::Unauthorized)
                    }
                }
                ApObject::Identifier(obj) => {
                    if obj.id == self.actor.clone().to_string() {
                        delete_actor(&conn, obj.id).await
                    } else {
                        log::debug!("DOESN'T MATCH ACTOR; ASSUMING NOTE");
                        if delete_note(&conn, obj.clone().id).await.is_ok() {
                            delete_timeline(&conn, obj.id).await
                        } else {
                            Err(Status::new(521))
                        }
                    }
                }
                _ => {
                    log::debug!("delete didn't match anything");
                    Err(Status::NoContent)
                }
            },
            MaybeReference::Reference(ap_id) => {
                if ap_id == self.actor.clone().to_string() {
                    delete_actor(&conn, ap_id).await
                } else {
                    log::debug!("DOESN'T MATCH ACTOR; ASSUMING NOTE");
                    if delete_note(&conn, ap_id.clone()).await.is_ok() {
                        delete_timeline(&conn, ap_id).await
                    } else {
                        Err(Status::new(522))
                    }
                }
            }
            _ => {
                log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
                Err(Status::NotImplemented)
            }
        }
    }
}

impl Outbox for Box<ApDelete> {
    async fn outbox(
        &self,
        conn: Db,
        events: EventChannels,
        profile: Profile,
    ) -> Result<String, Status> {
        outbox(conn, events, *self.clone(), profile).await
    }
}

async fn outbox(
    conn: Db,
    channels: EventChannels,
    delete: ApDelete,
    profile: Profile,
) -> Result<String, Status> {
    if let MaybeReference::Reference(id) = delete.object {
        if let Some(NoteLike::Note(note)) = get_notey(&conn, id).await {
            let activity = create_activity(
                Some(&conn),
                NewActivity::from((
                    Some(note.clone()),
                    None,
                    ActivityType::Delete,
                    ApAddress::Address(get_ap_id_from_username(profile.username.clone())),
                ))
                .link_profile(&conn)
                .await,
            )
            .await
            .map_err(|_| Status::new(520))?;

            runner::run(
                runner::note::delete_note_task,
                Some(conn),
                Some(channels),
                vec![activity.uuid.clone()],
            )
            .await;
            // let _ = to_faktory(faktory, "delete_note", vec![activity.uuid.clone()])
            //     .map_err(|_| Status::new(522));
            Ok(get_activity_ap_id_from_uuid(activity.uuid))
        } else {
            Err(Status::new(520))
        }
    } else {
        log::error!("DELETE OBJECT IS NOT A REFERENCE");
        Err(Status::NoContent)
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

impl Outbox for ApTombstone {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Profile,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
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
        let id = note.id.clone().ok_or("ApNote must have an ID")?;
        let tombstone =
            ApTombstone::try_from(note.clone()).map_err(|_| "ApNote must have an ID")?;
        Ok(ApDelete {
            context: Some(ApContext::default()),
            actor: note.attributed_to.clone(),
            kind: ApDeleteType::Delete,
            id: Some(format!("{id}#delete")),
            object: MaybeReference::Actual(ApObject::Tombstone(tombstone)),
            signature: None,
            to: note.to,
            cc: note.cc,
        })
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
    pub async fn sign(mut self, _profile: Profile) -> Result<ApDelete, ()> {
        let document = serde_json::to_string(&self).unwrap();
        log::debug!("DOCUMENT TO BE SIGNED\n{document:#?}");

        //let private_key = RsaPrivateKey::from_pkcs8_pem(&profile.private_key).unwrap();
        //let signing_key = SigningKey::<Sha256>::new_with_prefix(private_key);

        //let mut rng = rand::thread_rng();
        //let signed_hash = signing_key.sign_with_rng(&mut rng, document.as_bytes());

        if let Some(mut signature) = self.signature {
            //signature.signature_value = Some(base64::encode(signed_hash.as_bytes()));
            signature.kind = Some(ApSignatureType::RsaSignature2017);
            self.signature = Some(signature);

            Ok(self)
        } else {
            Err(())
        }
    }
}
