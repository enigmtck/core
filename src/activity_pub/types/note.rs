use core::fmt;
use std::{collections::HashMap, fmt::Debug};

use super::Ephemeral;
use crate::{
    activity_pub::{
        ActivityPub, ApActor, ApAttachment, ApCollection, ApContext, ApImage, ApInstrument, ApTag,
    },
    db::Db,
    models::{
        actors::get_actor_by_as_id,
        cache::{cache_content, Cache},
        coalesced_activity::CoalescedActivity,
        from_serde,
        objects::Object,
        objects::ObjectType,
    },
    MaybeMultiple,
};
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

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

impl From<ApNoteType> for String {
    fn from(kind: ApNoteType) -> String {
        format!("{kind:#?}")
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
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub tag: MaybeMultiple<ApTag>,
    pub attributed_to: ApAddress,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: ApNoteType,
    pub to: MaybeMultiple<ApAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub published: String,
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub cc: MaybeMultiple<ApAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replies: Option<ApCollection>,
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub attachment: MaybeMultiple<ApAttachment>,
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
    pub async fn load_ephemeral(&mut self, conn: &Db) {
        if let Ok(actor) = get_actor_by_as_id(conn, self.attributed_to.to_string()).await {
            let mut ephemeral = self.ephemeral.clone().unwrap_or_default();
            ephemeral.attributed_to = Some(vec![actor.into()]);
            self.ephemeral = Some(ephemeral);
        }
    }
}

impl Cache for ApNote {
    async fn cache(&self, conn: &Db) -> &Self {
        log::debug!("Checking for attachments");
        for attachment in self.attachment.multiple() {
            log::debug!("Attachment\n{attachment:#?}");
            cache_content(conn, attachment.clone().try_into()).await;
        }

        log::debug!("Checking for tags");
        for tag in self.tag.multiple() {
            log::debug!("Tag\n{tag:#?}");
            cache_content(conn, tag.clone().try_into()).await;
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

impl Default for ApNote {
    fn default() -> ApNote {
        ApNote {
            context: Some(ApContext::default()),
            tag: MaybeMultiple::None,
            attributed_to: ApAddress::default(),
            id: None,
            kind: ApNoteType::Note,
            to: MaybeMultiple::Multiple(vec![]),
            url: None,
            published: ActivityPub::time(Utc::now()),
            cc: MaybeMultiple::None,
            replies: None,
            attachment: MaybeMultiple::None,
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

impl From<ApActor> for ApNote {
    fn from(actor: ApActor) -> Self {
        ApNote {
            tag: MaybeMultiple::Multiple(vec![]),
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
        let cc: MaybeMultiple<ApAddress> = coalesced.object_cc.into();
        let tag = coalesced.object_tag.into();
        let attributed_to = coalesced
            .object_attributed_to
            .and_then(from_serde)
            .ok_or_else(|| anyhow::anyhow!("object_attributed_to is None"))?;
        let in_reply_to = coalesced.object_in_reply_to.and_then(from_serde);
        let content = coalesced
            .object_content
            .ok_or_else(|| anyhow::anyhow!("object_content is None"))?;
        let conversation = coalesced.object_conversation;
        let attachment = coalesced.object_attachment.into();
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
                cc: object.as_cc.clone().into(),
                tag: object.as_tag.clone().into(),
                attributed_to: from_serde(
                    object.as_attributed_to.ok_or(anyhow!("no attributed_to"))?,
                )
                .ok_or(anyhow!("failed to convert from Value"))?,
                content: object.as_content.clone().ok_or(anyhow!("no content"))?,
                replies: object.as_replies.clone().and_then(from_serde),
                in_reply_to: object.as_in_reply_to.clone().and_then(from_serde),
                attachment: object.as_attachment.clone().into(),
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
