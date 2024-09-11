use core::fmt;
use std::{collections::HashMap, fmt::Debug};

use crate::{
    activity_pub::{
        ApActor, ApAttachment, ApCollection, ApContext, ApImage, ApInstruments, ApTag, Outbox,
    },
    db::Db,
    fairings::events::EventChannels,
    helper::{
        get_activity_ap_id_from_uuid, get_ap_id_from_username, get_note_ap_id_from_uuid,
        get_note_url_from_uuid,
    },
    models::{
        activities::{create_activity, ActivityType, NewActivity, NoteActivity},
        cache::{cache_content, Cache},
        from_serde, from_time,
        notes::{create_note, NewNote, Note, NoteLike, NoteType},
        pg::coalesced_activity::CoalescedActivity,
        profiles::Profile,
        remote_notes::RemoteNote,
        timeline::{ContextualizedTimelineItem, TimelineItem},
        vault::VaultItem,
    },
    runner, MaybeMultiple, ANCHOR_RE,
};
use chrono::{DateTime, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use webpage::{Webpage, WebpageOptions};

use super::actor::ApAddress;

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

impl From<NoteType> for ApNoteType {
    fn from(kind: NoteType) -> Self {
        match kind {
            NoteType::Note => ApNoteType::Note,
            NoteType::EncryptedNote => ApNoteType::EncryptedNote,
            NoteType::VaultNote => ApNoteType::VaultNote,
        }
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
    pub ephemeral_announces: Option<Vec<String>>,
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
    pub ephemeral_likes: Option<Vec<String>>,
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

    pub fn dedup(mut self) -> Self {
        if let Some(mut announces) = self.ephemeral_announces {
            announces.sort();
            announces.dedup();
            self.ephemeral_announces = Some(announces);
        }

        if let Some(mut likes) = self.ephemeral_likes {
            likes.sort();
            likes.dedup();
            self.ephemeral_likes = Some(likes);
        }

        if let Some(mut actors) = self.ephemeral_actors {
            actors.sort();
            actors.dedup();
            self.ephemeral_actors = Some(actors);
        }

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
        profile: Profile,
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
        }
    }
}

type IdentifiedVaultItem = (VaultItem, Profile);

impl From<IdentifiedVaultItem> for ApNote {
    fn from((vault, profile): IdentifiedVaultItem) -> Self {
        ApNote {
            kind: ApNoteType::VaultNote,
            attributed_to: {
                if vault.outbound {
                    ApAddress::Address(get_ap_id_from_username(profile.clone().username))
                } else {
                    ApAddress::Address(vault.clone().remote_actor)
                }
            },
            to: {
                if vault.outbound {
                    MaybeMultiple::Multiple(vec![ApAddress::Address(vault.remote_actor)])
                } else {
                    MaybeMultiple::Multiple(vec![ApAddress::Address(get_ap_id_from_username(
                        profile.username,
                    ))])
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

impl TryFrom<&TimelineItem> for ApNote {
    type Error = anyhow::Error;

    fn try_from(timeline: &TimelineItem) -> Result<Self, Self::Error> {
        ApNote::try_from(timeline.clone())
    }
}

impl TryFrom<TimelineItem> for ApNote {
    type Error = anyhow::Error;

    fn try_from(item: TimelineItem) -> Result<Self, Self::Error> {
        ApNote::try_from(ContextualizedTimelineItem {
            item,
            ..Default::default()
        })
    }
}

impl TryFrom<ContextualizedTimelineItem> for ApNote {
    type Error = anyhow::Error;

    fn try_from(
        ContextualizedTimelineItem {
            item,
            activity,
            cc,
            related,
            requester,
        }: ContextualizedTimelineItem,
    ) -> Result<Self, Self::Error> {
        if item.kind.to_string().to_lowercase().as_str() == "note" {
            Ok(ApNote {
                context: Some(ApContext::default()),
                to: MaybeMultiple::Multiple(vec![]),
                cc: None,
                instrument: None,
                kind: ApNoteType::Note,
                tag: item.tag.and_then(from_serde),
                attributed_to: ApAddress::Address(item.attributed_to),
                id: Some(item.ap_id),
                url: item.url,
                published: item.published.unwrap_or("".to_string()),
                replies: None,
                in_reply_to: item.in_reply_to,
                content: item.content.unwrap_or_default(),
                summary: item.summary,
                sensitive: item.ap_sensitive,
                atom_uri: item.atom_uri,
                in_reply_to_atom_uri: item.in_reply_to_atom_uri,
                conversation: item.conversation,
                content_map: item.content_map.and_then(from_serde),
                attachment: item.attachment.and_then(from_serde),
                ephemeral_announces: Some(
                    activity
                        .iter()
                        .clone()
                        .filter(|activity| {
                            activity.kind.clone() == ActivityType::Announce && !activity.revoked
                        })
                        .map(|announce| announce.actor.clone())
                        .collect(),
                ),
                ephemeral_announced: {
                    let requester_ap_id = requester
                        .clone()
                        .map(|r| get_ap_id_from_username(r.username));
                    activity
                        .iter()
                        .find(|x| {
                            x.kind.clone() == ActivityType::Announce
                                && !x.revoked
                                && Some(x.actor.clone()) == requester_ap_id.clone()
                        })
                        .map(|x| get_activity_ap_id_from_uuid(x.uuid.clone()))
                },
                ephemeral_actors: Some(related),
                ephemeral_liked: {
                    let requester_ap_id = requester
                        .as_ref()
                        .map(|r| get_ap_id_from_username(r.username.clone()));
                    activity
                        .iter()
                        .find(|x| {
                            x.kind.clone() == ActivityType::Like
                                && !x.revoked
                                && Some(x.actor.clone()) == requester_ap_id.clone()
                        })
                        .map(|x| get_activity_ap_id_from_uuid(x.uuid.clone()))
                },
                ephemeral_likes: Some(
                    activity
                        .iter()
                        .filter(|activity| {
                            activity.kind.clone() == ActivityType::Like && !activity.revoked
                        })
                        .map(|like| like.actor.clone())
                        .collect(),
                ),
                ephemeral_targeted: Some(!cc.is_empty()),
                ephemeral_timestamp: from_time(item.created_at),
                ephemeral_metadata: item.metadata.and_then(from_serde),
            })
        } else {
            log::debug!("failed to convert ContextualizedTimelineItem to ApNote\n{item:#?}");
            Err(anyhow::Error::msg("wrong timeline_item type"))
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

impl From<NewNote> for ApNote {
    fn from(note: NewNote) -> Self {
        ApNote {
            tag: note.tag.and_then(from_serde),
            attributed_to: ApAddress::Address(note.attributed_to),
            published: Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            id: Some(format!(
                "https://{}/notes/{}",
                *crate::SERVER_NAME,
                note.uuid
            )),
            kind: note.kind.into(),
            to: from_serde(note.ap_to).unwrap(),
            content: note.content,
            cc: note.cc.and_then(from_serde),
            in_reply_to: note.in_reply_to,
            conversation: note.conversation,
            attachment: note.attachment.and_then(from_serde),
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

        let id = coalesced.object_id;
        let url = coalesced.object_url;
        let to = coalesced
            .object_to
            .and_then(from_serde)
            .ok_or_else(|| anyhow::anyhow!("object_to is None"))?;
        let cc = coalesced.object_cc.and_then(from_serde);
        let tag = coalesced.object_tag.and_then(from_serde);
        let attributed_to = ApAddress::from(
            coalesced
                .object_attributed_to
                .ok_or_else(|| anyhow::anyhow!("object_attributed_to is None"))?,
        );
        let in_reply_to = coalesced.object_in_reply_to;
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
            published,
            ..Default::default()
        })
    }
}

impl From<Note> for ApNote {
    fn from(note: Note) -> Self {
        cfg_if::cfg_if! {
            if #[cfg(feature = "pg")] {
                ApNote {
                    tag: serde_json::from_value(note.tag.into()).ok(),
                    attributed_to: ApAddress::Address(note.attributed_to),
                    published: note.updated_at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
                    id: note
                        .ap_id
                        .clone()
                        .map_or(Some(get_note_ap_id_from_uuid(note.uuid.clone())), Some),
                    url: Some(get_note_url_from_uuid(note.uuid)),
                    kind: note.kind.into(),
                    to: match serde_json::from_value(note.ap_to) {
                        Ok(x) => x,
                        Err(_) => MaybeMultiple::Multiple(vec![]),
                    },
                    content: note.content,
                    cc: match serde_json::from_value(note.cc.into()) {
                        Ok(x) => x,
                        Err(_) => Option::None,
                    },
                    in_reply_to: note.in_reply_to,
                    conversation: note.conversation,
                    attachment: note.attachment.map(|x| serde_json::from_value(x).unwrap()),
                    ephemeral_metadata: Some(vec![]),
                    ..Default::default()
                }
            } else if #[cfg(feature = "sqlite")] {
                ApNote {
                    tag: note
                        .tag
                        .as_deref()
                        .and_then(|x| serde_json::from_str(x).ok()),
                    attributed_to: ApAddress::Address(note.attributed_to),
                    published: note.updated_at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
                    id: note
                        .ap_id
                        .clone()
                        .map_or(Some(get_note_ap_id_from_uuid(note.uuid.clone())), Some),
                    url: Some(get_note_url_from_uuid(note.uuid)),
                    kind: note.kind.try_into().expect("failed to decode kind"),
                    to: match serde_json::from_str(&note.ap_to) {
                        Ok(x) => x,
                        Err(_) => MaybeMultiple::Multiple(vec![]),
                    },
                    content: note.content,
                    cc: note.cc.and_then(|x| serde_json::from_str(&x).ok()),
                    in_reply_to: note.in_reply_to,
                    conversation: note.conversation,
                    attachment: note.attachment.and_then(|x| serde_json::from_str(&x).ok()),
                    ephemeral_metadata: Some(vec![]),
                    ..Default::default()
                }
            }
        }
    }
}

// TODO: This is problematic for links that point to large files; the filter tries
// to account for some of that, but that's not really a solution. Maybe a whitelist?
// That would suck. I wish the Webpage crate had a size limit (i.e., load pages with
// a maximum size of 10MB or whatever a reasonable amount would be).
fn get_links(text: String) -> Vec<String> {
    ANCHOR_RE
        .captures_iter(&text)
        .filter(|cap| {
            !cap[0].to_lowercase().contains("mention")
                && !cap[0].to_lowercase().contains("u-url")
                && !cap[0].to_lowercase().contains("hashtag")
                && !cap[0].to_lowercase().contains("download")
                && !cap[1].to_lowercase().contains(".pdf")
        })
        .map(|cap| cap[1].to_string())
        .collect()
}

fn metadata(remote_note: &RemoteNote) -> Vec<Metadata> {
    get_links(remote_note.content.clone())
        .iter()
        .map(|link| {
            (
                link.clone(),
                Webpage::from_url(link, WebpageOptions::default()),
            )
        })
        .filter(|(_, metadata)| metadata.is_ok())
        .map(|(link, metadata)| (link, metadata.unwrap().html.meta).into())
        .collect()
}

impl From<RemoteNote> for ApNote {
    fn from(remote_note: RemoteNote) -> ApNote {
        cfg_if::cfg_if! {
            if #[cfg(feature = "pg")] {
                let kind = match remote_note.kind {
                    NoteType::Note => ApNoteType::Note,
                    NoteType::EncryptedNote => ApNoteType::EncryptedNote,
                    _ => ApNoteType::default(),
                };

                ApNote {
                    id: Some(remote_note.ap_id.clone()),
                    kind,
                    published: remote_note.published.clone().unwrap_or("".to_string()),
                    url: remote_note.url.clone(),
                    to: match serde_json::from_value(remote_note.ap_to.clone().into()) {
                        Ok(x) => x,
                        Err(_) => MaybeMultiple::Multiple(vec![]),
                    },
                    cc: match serde_json::from_value(remote_note.cc.clone().into()) {
                        Ok(x) => x,
                        Err(_) => None,
                    },
                    tag: match serde_json::from_value(remote_note.tag.clone().into()) {
                        Ok(x) => x,
                        Err(_) => None,
                    },
                    attributed_to: ApAddress::Address(remote_note.attributed_to.clone()),
                    content: remote_note.content.clone(),
                    replies: match serde_json::from_value(remote_note.replies.clone().into()) {
                        Ok(x) => x,
                        Err(_) => None,
                    },
                    in_reply_to: remote_note.in_reply_to.clone(),
                    attachment: match serde_json::from_value(
                        remote_note.attachment.clone().unwrap_or_default(),
                    ) {
                        Ok(x) => x,
                        Err(_) => None,
                    },
                    conversation: remote_note.conversation.clone(),
                    ephemeral_timestamp: Some(remote_note.created_at),
                    ephemeral_metadata: Some(metadata(&remote_note)),
                    ..Default::default()
                }
            } else if #[cfg(feature = "sqlite")] {
                let kind = match remote_note.kind.as_str() {
                    "note" => ApNoteType::Note,
                    "encrypted_note" => ApNoteType::EncryptedNote,
                    _ => ApNoteType::default(),
                };

                ApNote {
                    id: Some(remote_note.ap_id.clone()),
                    kind,
                    published: remote_note.published.clone().unwrap_or("".to_string()),
                    url: remote_note.url.clone(),
                    to: remote_note
                        .ap_to
                        .clone()
                        .as_deref()
                        .and_then(|x| serde_json::from_str(x).ok())
                        .expect("no to in RemoteNote"),
                    cc: remote_note
                        .cc
                        .clone()
                        .as_deref()
                        .and_then(|x| serde_json::from_str(x).ok()),
                    tag: remote_note
                        .tag
                        .clone()
                        .as_deref()
                        .and_then(|x| serde_json::from_str(x).ok()),
                    attributed_to: ApAddress::Address(remote_note.attributed_to.clone()),
                    content: remote_note.content.clone(),
                    replies: remote_note
                        .replies
                        .clone()
                        .as_deref()
                        .and_then(|x| serde_json::from_str(x).ok()),
                    in_reply_to: remote_note.in_reply_to.clone(),
                    attachment: remote_note
                        .attachment
                        .clone()
                        .as_deref()
                        .and_then(|x| serde_json::from_str(x).ok()),
                    conversation: remote_note.conversation.clone(),
                    ephemeral_timestamp: Some(DateTime::<Utc>::from_naive_utc_and_offset(
                        remote_note.created_at,
                        Utc,
                    )),
                    ephemeral_metadata: Some(metadata(&remote_note)),
                    ..Default::default()
                }

            }
        }
    }
}

async fn handle_note(
    conn: Db,
    channels: EventChannels,
    mut note: ApNote,
    profile: Profile,
) -> Result<String, Status> {
    // ApNote -> NewNote -> ApNote -> ApActivity
    // UUID is set in NewNote

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

    let created_note = create_note(&conn, NewNote::from((note.clone(), profile.id)))
        .await
        .ok_or(Status::new(520))?;

    let activity = create_activity(
        Some(&conn),
        NewActivity::from(NoteActivity {
            note: NoteLike::Note(created_note.clone()),
            profile,
            kind: ActivityType::Create,
        })
        .link_profile(&conn)
        .await,
    )
    .await
    .map_err(|_| Status::new(521))?;

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
    profile: Profile,
) -> Result<String, Status> {
    // ApNote -> NewNote -> ApNote -> ApActivity
    // UUID is set in NewNote
    let n = NewNote::from((note.clone(), profile.id));

    if let Some(created_note) = create_note(&conn, n.clone()).await {
        log::debug!("created_note\n{created_note:#?}");

        runner::run(
            runner::note::outbound_note_task,
            Some(conn),
            Some(channels),
            vec![created_note.uuid.clone()],
        )
        .await;
        Ok(created_note.uuid)
    } else {
        Err(Status::NoContent)
    }
}
