use core::fmt;
use std::{collections::HashMap, fmt::Debug};

use crate::{
    activity_pub::{
        ApActor, ApAttachment, ApCollection, ApContext, ApImage, ApInstruments, ApTag, Outbox,
    },
    db::Db,
    fairings::events::EventChannels,
    helper::{get_note_ap_id_from_uuid, get_note_url_from_uuid},
    models::{
        activities::{create_activity, NewActivity},
        actors::Actor,
        cache::{cache_content, Cache},
        from_serde, from_time,
        objects::{create_or_update_object, Object},
        pg::{coalesced_activity::CoalescedActivity, objects::ObjectType},
        vault::VaultItem,
    },
    runner, MaybeMultiple,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{actor::ApActorTerse, actor::ApAddress, create::ApCreate, object::ApObject};

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
        if kind.is_note() {
            Ok(ApNoteType::Note)
        } else {
            Err(anyhow!("invalid Object type for ApNote"))
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    //pub to: Vec<String>,
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
    pub atom_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to_atom_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_map: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instrument: Option<ApInstruments>,

    // These are ephemeral attributes to facilitate client operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_announces: Option<Vec<ApActorTerse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_actors: Option<Vec<ApActor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_liked: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_announced: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_targeted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_timestamp: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_metadata: Option<Vec<Metadata>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_likes: Option<Vec<ApActorTerse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_attributed_to: Option<Vec<ApActorTerse>>,

    #[serde(skip_serializing)]
    pub internal_uuid: Option<String>,
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

        if let Some(metadata_vec) = self.ephemeral_metadata.clone() {
            for metadata in metadata_vec {
                if let Some(og_image) = metadata.og_image.clone() {
                    cache_content(conn, Ok(ApImage::from(og_image).into())).await;
                }

                if let Some(twitter_image) = metadata.twitter_image.clone() {
                    cache_content(conn, Ok(ApImage::from(twitter_image).into())).await;
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
        match self.kind {
            ApNoteType::Note => handle_note(conn, events, self.clone(), profile).await,
            ApNoteType::EncryptedNote => {
                handle_encrypted_note(conn, events, self.clone(), profile).await
            }
            _ => Err(Status::NoContent),
        }
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
            published: Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            cc: None,
            replies: None,
            attachment: None,
            in_reply_to: None,
            content: String::new(),
            summary: None,
            sensitive: None,
            atom_uri: None,
            in_reply_to_atom_uri: None,
            conversation: None,
            content_map: None,
            instrument: None,
            ephemeral_announces: None,
            ephemeral_actors: None,
            ephemeral_liked: None,
            ephemeral_announced: None,
            ephemeral_targeted: None,
            ephemeral_timestamp: None,
            ephemeral_metadata: None,
            ephemeral_likes: None,
            ephemeral_attributed_to: None,
            internal_uuid: None,
        }
    }
}

type IdentifiedVaultItem = (VaultItem, Actor);

impl From<IdentifiedVaultItem> for ApNote {
    fn from((vault, profile): IdentifiedVaultItem) -> Self {
        ApNote {
            kind: ApNoteType::VaultNote,
            attributed_to: {
                if vault.outbound {
                    ApAddress::Address(profile.clone().as_id)
                } else {
                    ApAddress::Address(vault.clone().remote_actor)
                }
            },
            to: {
                if vault.outbound {
                    MaybeMultiple::Multiple(vec![ApAddress::Address(vault.remote_actor)])
                } else {
                    MaybeMultiple::Multiple(vec![ApAddress::Address(profile.as_id)])
                }
            },
            id: Some(format!(
                "https://{}/vault/{}",
                *crate::SERVER_NAME,
                vault.uuid
            )),
            content: vault.encrypted_data,
            published: from_time(vault.created_at).unwrap().to_rfc3339(),
            ..Default::default()
        }
    }
}

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
        let ephemeral_metadata = coalesced.object_metadata.and_then(from_serde);
        let ephemeral_announces = from_serde(coalesced.object_announcers);
        let ephemeral_likes = from_serde(coalesced.object_likers);
        let ephemeral_announced = coalesced.object_announced;
        let ephemeral_liked = coalesced.object_liked;
        let ephemeral_attributed_to = from_serde(coalesced.object_attributed_to_profiles);

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
            published: published.to_rfc2822(),
            ephemeral_metadata,
            ephemeral_announces,
            ephemeral_likes,
            ephemeral_announced,
            ephemeral_liked,
            ephemeral_attributed_to,
            ..Default::default()
        })
    }
}

impl TryFrom<Object> for ApNote {
    type Error = anyhow::Error;

    fn try_from(object: Object) -> Result<ApNote> {
        if object.as_type.is_note() {
            Ok(ApNote {
                id: Some(object.as_id.clone()),
                kind: ApNoteType::Note,
                published: object.as_published.unwrap_or(Utc::now()).to_rfc2822(),
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
                ephemeral_timestamp: Some(object.created_at),
                ephemeral_metadata: object.ek_metadata.and_then(from_serde),
                ..Default::default()
            })
        } else {
            Err(anyhow!("ObjectType is not Note"))
        }
    }
}

async fn handle_note(
    conn: Db,
    channels: EventChannels,
    mut note: ApNote,
    profile: Actor,
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
    note.internal_uuid = Some(uuid.clone());
    note.id = Some(get_note_ap_id_from_uuid(uuid.clone()));
    note.url = Some(get_note_url_from_uuid(uuid.clone()));
    note.published = Utc::now().to_rfc3339();
    note.attributed_to = profile.as_id.into();

    let object = create_or_update_object(&conn, note.into())
        .await
        .map_err(|_| Status::InternalServerError)?;

    let create = ApCreate::try_from(ApObject::Note(
        ApNote::try_from(object.clone()).map_err(|_| Status::InternalServerError)?,
    ))
    .map_err(|_| Status::InternalServerError)?;

    let activity = create_activity(
        Some(&conn),
        NewActivity::try_from((create.into(), Some(object.into())))
            .map_err(|_| Status::InternalServerError)?
            .link_actor(&conn)
            .await,
    )
    .await
    .map_err(|_| Status::InternalServerError)?;

    runner::run(
        runner::note::outbound_note_task,
        Some(conn),
        Some(channels),
        vec![activity.uuid.clone()],
    )
    .await;
    Ok(activity.uuid)
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
        .map_err(|_| Status::InternalServerError)?;

    log::debug!("created_note\n{object:#?}");

    let ek_uuid = object.ek_uuid.ok_or(Status::InternalServerError)?;

    runner::run(
        runner::note::outbound_note_task,
        Some(conn),
        Some(channels),
        vec![ek_uuid.clone()],
    )
    .await;
    Ok(ek_uuid)
}
