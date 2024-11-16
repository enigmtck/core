use core::fmt;
use std::{collections::HashMap, fmt::Debug};

use super::Ephemeral;
use crate::{
    activity_pub::ApActivity,
    activity_pub::{
        ActivityPub, ApActor, ApAttachment, ApCollection, ApContext, ApImage, ApInstrument, ApTag,
        Outbox,
    },
    db::Db,
    fairings::events::EventChannels,
    helper::get_instrument_as_id_from_uuid,
    helper::{
        get_conversation_ap_id_from_uuid, get_object_ap_id_from_uuid, get_object_url_from_uuid,
    },
    models::activities::get_activity_by_ap_id,
    models::{
        activities::{create_activity, NewActivity},
        actors::{get_actor, Actor},
        cache::{cache_content, Cache, Cacheable},
        from_serde, from_time,
        objects::{create_or_update_object, Object},
        olm_sessions::create_or_update_olm_session,
        pg::{
            coalesced_activity::CoalescedActivity, objects::ObjectType, vault::create_vault_item,
        },
        vault::VaultItem,
    },
    runner::{
        self,
        //encrypted::handle_encrypted_note,
        get_inboxes,
        send_to_inboxes,
        TaskError,
    },
    MaybeMultiple,
};
use anyhow::{anyhow, Result};
use chrono::Utc;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{actor::ApAddress, create::ApCreate, object::ApObject};

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub enum ApNoteType {
    #[default]
    #[serde(alias = "note")]
    Note,
    #[serde(alias = "encrypted_note")]
    EncryptedNote,
    #[serde(alias = "vault_note")]
    VaultNote,
    // #[serde(alias = "question")]
    // Question,
}

impl fmt::Display for ApNoteType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl TryFrom<String> for ApNoteType {
    type Error = &'static str;

    fn try_from(kind: String) -> Result<Self, Self::Error> {
        match kind.as_str() {
            "note" => Ok(ApNoteType::Note),
            "encrypted_note" => Ok(ApNoteType::EncryptedNote),
            "vault_note" => Ok(ApNoteType::VaultNote),
            _ => Err("no match for {kind}"),
        }
    }
}

impl TryFrom<ObjectType> for ApNoteType {
    type Error = anyhow::Error;

    fn try_from(kind: ObjectType) -> Result<Self, Self::Error> {
        match kind {
            ObjectType::Note => Ok(Self::Note),
            ObjectType::EncryptedNote => Ok(Self::EncryptedNote),
            _ => Err(anyhow!("invalid Object type for ApNote")),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub url: String,
    pub twitter_title: Option<String>,
    pub description: Option<String>,
    pub og_description: Option<String>,
    pub og_title: Option<String>,
    pub og_image: Option<String>,
    pub og_site_name: Option<String>,
    pub twitter_image: Option<String>,
    pub og_url: Option<String>,
    pub twitter_description: Option<String>,
    pub published: Option<String>,
    pub twitter_site: Option<String>,
    pub og_type: Option<String>,
}

// (Base Site URL, Metadata Hashmap)
type SiteMetadata = (String, HashMap<String, String>);
impl From<SiteMetadata> for Metadata {
    fn from((url, meta): SiteMetadata) -> Self {
        fn sanitize(url: &str, path: Option<String>) -> Option<String> {
            const MAX_URL_LENGTH: usize = 2048; // Define a reasonable max length for a URL

            path.filter(|p| {
                // Check the scheme and host to avoid security-sensitive URLs
                !(p.starts_with("http://localhost") ||
                  p.starts_with("https://localhost") ||
                  p.starts_with("http://127.0.0.1") ||
                  p.starts_with("https://127.0.0.1") ||
                  p.starts_with("http://0.0.0.0") ||
                  p.starts_with("https://0.0.0.0") ||
                  p.starts_with("file:///") ||
                  p.starts_with("javascript:") ||
                  p.starts_with("data:") ||
                  // Add checks for other IP ranges or conditions as needed
                  p.len() > MAX_URL_LENGTH)
            })
            .map(
                |p| match p.starts_with("http://") || p.starts_with("https://") {
                    true => p,
                    false => format!(
                        "{}/{}",
                        url.trim_end_matches('/'),
                        p.trim_start_matches('/')
                    ),
                },
            )
        }

        let og_image = sanitize(&url, meta.get("og:image").cloned());
        let twitter_image = sanitize(&url, meta.get("twitter:image").cloned());

        Metadata {
            url,
            twitter_title: meta.get("twitter:title").cloned(),
            description: meta.get("description").cloned(),
            og_description: meta.get("og:description").cloned(),
            og_title: meta.get("og:title").cloned(),
            og_image,
            og_site_name: meta.get("og:site_name").cloned(),
            twitter_image,
            og_url: meta.get("og:url").cloned(),
            twitter_description: meta.get("twitter:description").cloned(),
            published: meta.get("article:published").cloned(),
            twitter_site: meta.get("twitter:site").cloned(),
            og_type: meta.get("og:type").cloned(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApNote {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    pub tag: Option<Vec<ApTag>>,
    pub attributed_to: ApAddress,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: ApNoteType,
    pub to: MaybeMultiple<ApAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub published: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<MaybeMultiple<ApAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replies: Option<ApCollection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment: Option<Vec<ApAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<String>,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_map: Option<HashMap<String, String>>,

    // We skip serializing here because 'instrument' doesn't belong
    // on a Note object; it's here only to facilitate the Outbox action
    // to move it to the Create activity.
    #[serde(skip_serializing)]
    pub instrument: Option<MaybeMultiple<ApInstrument>>,

    // These are ephemeral attributes to facilitate client operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<Ephemeral>,
}

impl ApNote {
    pub fn to(mut self, to: String) -> Self {
        if let MaybeMultiple::Multiple(v) = self.to {
            let mut t = v;
            t.push(ApAddress::Address(to));
            self.to = MaybeMultiple::Multiple(t);
        }
        self
    }

    pub fn content(mut self, content: String) -> Self {
        self.content = content;
        self
    }

    pub fn tag(mut self, tag: ApTag) -> Self {
        self.tag.as_mut().expect("unwrap failed").push(tag);
        self
    }

    async fn outbox_note(
        conn: Db,
        channels: EventChannels,
        mut note: ApNote,
        profile: Actor,
        raw: Value,
    ) -> Result<String, Status> {
        // ApNote -> NewNote -> ApNote -> ApActivity
        // UUID is set in NewNote

        if note.id.is_some() {
            return Err(Status::NotAcceptable);
        }

        let mut is_public = false;
        let mut followers_included = false;
        let mut addresses_cc: Vec<ApAddress> = note.cc.clone().unwrap_or(vec![].into()).multiple();
        let followers = ApActor::from(profile.clone()).followers;

        if let Some(followers) = followers {
            // look for the public and followers group address aliases in the to vec
            for to in note.to.multiple().iter() {
                if to.is_public() {
                    is_public = true;
                    if to.to_string().to_lowercase() == followers.to_lowercase() {
                        followers_included = true;
                    }
                }
            }

            // look for the public and followers group address aliases in the cc vec
            for cc in addresses_cc.iter() {
                if cc.is_public() {
                    is_public = true;
                    if cc.to_string().to_lowercase() == followers.to_lowercase() {
                        followers_included = true;
                    }
                }
            }

            // if the note is public and if it's not already included, add the sender's followers group
            if is_public && !followers_included {
                addresses_cc.push(followers.into());
                note.cc = Some(MaybeMultiple::Multiple(addresses_cc));
            }
        }

        let uuid = Uuid::new_v4().to_string();
        note.ephemeral = Some(Ephemeral {
            internal_uuid: Some(uuid.clone()),
            ..Default::default()
        });
        note.id = Some(get_object_ap_id_from_uuid(uuid.clone()));
        note.url = Some(get_object_url_from_uuid(uuid.clone()));
        note.published = ActivityPub::time(Utc::now());
        note.attributed_to = profile.as_id.clone().into();

        if note.conversation.is_none() {
            note.conversation = Some(get_conversation_ap_id_from_uuid(Uuid::new_v4().to_string()));
        }

        note.context = Some(ApContext::default());

        // Setting the ID for the instruments here to use in the ApCreate further down.
        // The UUID will be overwritten for the VaultItem when that is stored to the database.
        let instruments = note
            .instrument
            .clone()
            .map(|mut instrument| match &mut instrument {
                MaybeMultiple::Single(inst) => {
                    let uuid = Uuid::new_v4().to_string();
                    inst.uuid = Some(uuid.clone());
                    inst.id = Some(get_instrument_as_id_from_uuid(uuid));
                    vec![inst.clone()]
                }
                MaybeMultiple::Multiple(insts) => {
                    insts.iter_mut().for_each(|inst| {
                        let uuid = Uuid::new_v4().to_string();
                        inst.uuid = Some(uuid.clone());
                        inst.id = Some(get_instrument_as_id_from_uuid(uuid))
                    });
                    insts.clone()
                }
                _ => vec![],
            })
            .unwrap_or_default();

        note.instrument = if instruments.is_empty() {
            None
        } else {
            Some(instruments.clone().into())
        };

        for instrument in instruments.clone() {
            if instrument.is_olm_session() {
                create_or_update_olm_session(
                    &conn,
                    (
                        instrument.clone(),
                        note.attributed_to.to_string(),
                        note.to.single().unwrap().to_string(),
                    )
                        .try_into()
                        .unwrap(),
                )
                .await;
            };
        }

        let object = create_or_update_object(&conn, (note.clone(), profile).into())
            .await
            .map_err(|e| {
                log::error!("FAILED TO CREATE OR UPDATE OBJECT: {e:#?}");
                Status::InternalServerError
            })?;

        let create = ApCreate::try_from(ApObject::Note(ApNote::try_from(object.clone()).map_err(
            |e| {
                log::error!("FAILED TO BUILD AP_NOTE: {e:#?}");
                Status::InternalServerError
            },
        )?))
        .map_err(|e| {
            log::error!("FAILED TO BUILD AP_CREATE: {e:#?}");
            Status::InternalServerError
        })?;

        let mut activity = NewActivity::try_from((create.into(), Some(object.into())))
            .map_err(|e| {
                log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                Status::InternalServerError
            })?
            .link_actor(&conn)
            .await;

        activity.raw = Some(raw);

        let activity = create_activity(Some(&conn), activity.clone())
            .await
            .map_err(|e| {
                log::error!("FAILED TO CREATE ACTIVITY: {e:#?}");
                Status::InternalServerError
            })?;

        for instrument in instruments.clone() {
            if instrument.is_vault_item() {
                create_vault_item(
                    &conn,
                    (
                        instrument.content.unwrap(),
                        note.attributed_to.to_string(),
                        activity.id,
                    )
                        .into(),
                )
                .await;
            }
        }

        runner::run(
            ApNote::send_note,
            Some(conn),
            Some(channels),
            vec![activity.ap_id.clone().ok_or_else(|| {
                log::error!("ACTIVITY AP_ID CAN NOT BE NONE");
                Status::InternalServerError
            })?],
        )
        .await;
        Ok(activity.uuid)
    }

    async fn send_note(
        conn: Option<Db>,
        _channels: Option<EventChannels>,
        ap_ids: Vec<String>,
    ) -> Result<(), TaskError> {
        let conn = conn.as_ref();

        for ap_id in ap_ids {
            let (activity, target_activity, target_object, target_actor) =
                get_activity_by_ap_id(conn.ok_or(TaskError::TaskFailed)?, ap_id.clone())
                    .await
                    .ok_or_else(|| {
                        log::error!("FAILED TO RETRIEVE ACTIVITY");
                        TaskError::TaskFailed
                    })?;

            let profile_id = activity.actor_id.ok_or_else(|| {
                log::error!("ACTIVITY ACTOR_ID CAN NOT BE NONE");
                TaskError::TaskFailed
            })?;

            let sender = get_actor(conn.unwrap(), profile_id).await.ok_or_else(|| {
                log::error!("FAILED TO RETRIEVE SENDER ACTOR");
                TaskError::TaskFailed
            })?;

            let note = ApNote::try_from(target_object.clone().ok_or(TaskError::TaskFailed)?)
                .map_err(|e| {
                    log::error!("FAILED TO BUILD ApNote: {e:#?}");
                    TaskError::TaskFailed
                })?;

            // For the Svelte client, all images are passed through the server cache. To cache an image
            // that's already on the server seems weird, but I think it's a better choice than trying
            // to handle the URLs for local images differently.
            //let ap_note: ApNote = note.clone().into();
            if let Some(attachments) = note.clone().attachment {
                for attachment in attachments {
                    if let ApAttachment::Document(document) = attachment {
                        let _ = cache_content(
                            conn.unwrap(),
                            Cacheable::try_from(ApImage::try_from(document)),
                        )
                        .await;
                    }
                }
            }

            cfg_if::cfg_if! {
                if #[cfg(feature = "pg")] {
                    let activity = match note.kind {
                        ApNoteType::Note | ApNoteType::EncryptedNote => {
                            if let Ok(activity) =
                                ApActivity::try_from((
                                    activity,
                                    target_activity,
                                    target_object,
                                    target_actor
                                )) {
                                    Some(activity)
                                } else {
                                    log::error!("FAILED TO BUILD ApActivity");
                                    None
                                }
                        }
                        // NoteType::EncryptedNote => {
                        //     handle_encrypted_note(&mut note, sender.clone())
                        //         .map(ApActivity::Create(ApCreate::from))
                        // }
                        _ => None,
                    };
                } else if #[cfg(feature = "sqlite")] {
                    let activity = {
                        if note.kind.to_lowercase().as_str() == "note" {
                            if let Ok(activity) = ApActivity::try_from((
                                (
                                    activity,
                                    target_activity,
                                    target_object
                                ),
                                None,
                            )) {
                                Some(activity)
                            } else {
                                None
                            }
                        } else {
                            // NoteType::EncryptedNote => {
                            //     handle_encrypted_note(&mut note, sender.clone())
                            //         .map(ApActivity::Create(ApCreate::from))
                            // }
                            None
                        }
                    };
                }
            }

            let _ = runner::actor::get_actor(
                conn,
                sender.clone(),
                note.clone().attributed_to.to_string(),
            )
            .await;

            let activity = activity.ok_or_else(|| {
                log::error!("ACTIVITY CANNOT BE NONE");
                TaskError::TaskFailed
            })?;

            let inboxes: Vec<ApAddress> = get_inboxes(conn, activity.clone(), sender.clone()).await;

            log::debug!("SENDING ACTIVITY\n{activity:#?}");
            log::debug!("SENDER\n{sender:#?}");
            log::debug!("INBOXES\n{inboxes:#?}");

            log::debug!("SKIPPING SEND FOR DEBUG");
            // send_to_inboxes(conn.unwrap(), inboxes, sender, activity)
            //     .await
            //     .map_err(|e| {
            //         log::error!("FAILED TO SEND ACTIVITY: {e:#?}");
            //         TaskError::TaskFailed
            //     })?;
        }

        Ok(())
    }
}

impl Cache for ApNote {
    async fn cache(&self, conn: &Db) -> &Self {
        if let Some(attachments) = self.attachment.clone() {
            for attachment in attachments {
                cache_content(conn, attachment.clone().try_into()).await;
            }
        }

        if let Some(tags) = self.tag.clone() {
            for tag in tags {
                cache_content(conn, tag.clone().try_into()).await;
            }
        }

        if let Some(ephemeral) = self.ephemeral.clone() {
            if let Some(metadata_vec) = ephemeral.metadata.clone() {
                for metadata in metadata_vec {
                    if let Some(og_image) = metadata.og_image.clone() {
                        cache_content(conn, Ok(ApImage::from(og_image).into())).await;
                    }

                    if let Some(twitter_image) = metadata.twitter_image.clone() {
                        cache_content(conn, Ok(ApImage::from(twitter_image).into())).await;
                    }
                }
            }
        }

        self
    }
}

impl Outbox for ApNote {
    async fn outbox(
        &self,
        conn: Db,
        events: EventChannels,
        profile: Actor,
        raw: Value,
    ) -> Result<String, Status> {
        ApNote::outbox_note(conn, events, self.clone(), profile, raw).await
    }
}

impl Default for ApNote {
    fn default() -> ApNote {
        ApNote {
            context: Some(ApContext::default()),
            tag: None,
            attributed_to: ApAddress::default(),
            id: None,
            kind: ApNoteType::Note,
            to: MaybeMultiple::Multiple(vec![]),
            url: None,
            published: ActivityPub::time(Utc::now()),
            cc: None,
            replies: None,
            attachment: None,
            in_reply_to: None,
            content: String::new(),
            summary: None,
            sensitive: None,
            conversation: None,
            content_map: None,
            instrument: None,
            ephemeral: None,
        }
    }
}

// type IdentifiedVaultItem = (VaultItem, Actor);

// impl From<IdentifiedVaultItem> for ApNote {
//     fn from((vault, profile): IdentifiedVaultItem) -> Self {
//         ApNote {
//             kind: ApNoteType::VaultNote,
//             attributed_to: {
//                 if vault.outbound {
//                     ApAddress::Address(profile.clone().as_id)
//                 } else {
//                     ApAddress::Address(vault.clone().remote_actor)
//                 }
//             },
//             to: {
//                 if vault.outbound {
//                     MaybeMultiple::Multiple(vec![ApAddress::Address(vault.remote_actor)])
//                 } else {
//                     MaybeMultiple::Multiple(vec![ApAddress::Address(profile.as_id)])
//                 }
//             },
//             id: Some(format!(
//                 "https://{}/vault/{}",
//                 *crate::SERVER_NAME,
//                 vault.uuid
//             )),
//             content: vault.encrypted_data,
//             published: ActivityPub::time(from_time(vault.created_at).unwrap()),
//             ..Default::default()
//         }
//     }
// }

impl From<ApActor> for ApNote {
    fn from(actor: ApActor) -> Self {
        ApNote {
            tag: Some(vec![]),
            attributed_to: actor.id.unwrap(),
            id: None,
            kind: ApNoteType::Note,
            to: MaybeMultiple::Multiple(vec![]),
            content: String::new(),
            ..Default::default()
        }
    }
}

impl TryFrom<CoalescedActivity> for ApNote {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        let kind = coalesced
            .object_type
            .ok_or_else(|| anyhow::anyhow!("object_type is None"))?
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to convert object_type: {}", e))?;

        let id = coalesced.object_as_id;
        let url = coalesced
            .object_url
            .and_then(from_serde::<MaybeMultiple<String>>)
            .and_then(|x| x.single().ok());
        let to = coalesced
            .object_to
            .and_then(from_serde)
            .ok_or_else(|| anyhow::anyhow!("object_to is None"))?;
        let cc = coalesced.object_cc.and_then(from_serde);
        let tag = coalesced.object_tag.and_then(from_serde);
        let attributed_to = coalesced
            .object_attributed_to
            .and_then(from_serde)
            .ok_or_else(|| anyhow::anyhow!("object_attributed_to is None"))?;
        let in_reply_to = coalesced.object_in_reply_to.and_then(from_serde);
        let content = coalesced
            .object_content
            .ok_or_else(|| anyhow::anyhow!("object_content is None"))?;
        let conversation = coalesced.object_conversation;
        let attachment = coalesced.object_attachment.and_then(from_serde);
        let summary = coalesced.object_summary;
        let sensitive = coalesced.object_sensitive;
        let published = coalesced
            .object_published
            .ok_or_else(|| anyhow::anyhow!("object_published is None"))?;
        let ephemeral = Some(Ephemeral {
            metadata: coalesced.object_metadata.and_then(from_serde),
            announces: from_serde(coalesced.object_announcers),
            likes: from_serde(coalesced.object_likers),
            announced: coalesced.object_announced,
            liked: coalesced.object_liked,
            attributed_to: from_serde(coalesced.object_attributed_to_profiles),
            ..Default::default()
        });
        let instrument = coalesced.object_instrument.and_then(from_serde);

        Ok(ApNote {
            kind,
            id,
            url,
            to,
            cc,
            tag,
            attributed_to,
            in_reply_to,
            content,
            conversation,
            attachment,
            summary,
            sensitive,
            published: ActivityPub::time(published),
            ephemeral,
            instrument,
            ..Default::default()
        })
    }
}

impl TryFrom<Object> for ApNote {
    type Error = anyhow::Error;

    fn try_from(object: Object) -> Result<ApNote> {
        if object.as_type.is_note() || object.as_type.is_encrypted_note() {
            Ok(ApNote {
                id: Some(object.as_id.clone()),
                kind: object.as_type.try_into()?,
                published: ActivityPub::time(object.as_published.unwrap_or(Utc::now())),
                url: object.as_url.clone().and_then(from_serde),
                to: object
                    .as_to
                    .clone()
                    .and_then(from_serde)
                    .unwrap_or(vec![].into()),
                cc: object.as_cc.clone().and_then(from_serde),
                tag: object.as_tag.clone().and_then(from_serde),
                attributed_to: from_serde(
                    object.as_attributed_to.ok_or(anyhow!("no attributed_to"))?,
                )
                .ok_or(anyhow!("failed to convert from Value"))?,
                content: object.as_content.clone().ok_or(anyhow!("no content"))?,
                replies: object.as_replies.clone().and_then(from_serde),
                in_reply_to: object.as_in_reply_to.clone().and_then(from_serde),
                attachment: match serde_json::from_value(
                    object.as_attachment.clone().unwrap_or_default(),
                ) {
                    Ok(x) => x,
                    Err(_) => None,
                },
                conversation: object.ap_conversation.clone(),
                ephemeral: Some(Ephemeral {
                    timestamp: Some(object.created_at),
                    metadata: object.ek_metadata.and_then(from_serde),
                    ..Default::default()
                }),
                instrument: object.ek_instrument.clone().and_then(from_serde),
                ..Default::default()
            })
        } else {
            Err(anyhow!("ObjectType is not Note"))
        }
    }
}

async fn handle_encrypted_note(
    conn: Db,
    channels: EventChannels,
    note: ApNote,
    _profile: Actor,
) -> Result<String, Status> {
    // ApNote -> NewNote -> ApNote -> ApActivity
    // UUID is set in NewNote

    let object = create_or_update_object(&conn, note.into())
        .await
        .map_err(|e| {
            log::error!("FAILED TO CREATE Object: {e:#?}");
            Status::InternalServerError
        })?;

    let ek_uuid = object.ek_uuid.ok_or_else(|| {
        log::error!("MISSING ed_uuid");
        Status::InternalServerError
    })?;

    runner::run(
        ApNote::send_note,
        Some(conn),
        Some(channels),
        vec![ek_uuid.clone()],
    )
    .await;
    Ok(ek_uuid)
}
