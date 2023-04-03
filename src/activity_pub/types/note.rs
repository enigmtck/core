use core::fmt;
use std::{collections::HashMap, fmt::Debug};

use crate::{
    activity_pub::{ApActor, ApAttachment, ApCollection, ApContext, ApInstruments, ApTag},
    helper::get_ap_id_from_username,
    models::{
        likes::Like,
        notes::{NewNote, Note},
        profiles::Profile,
        remote_notes::RemoteNote,
        timeline::{TimelineItem, TimelineItemCc},
        vault::VaultItem,
    },
    MaybeMultiple,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApNoteType {
    Note,
    EncryptedNote,
    VaultNote,
    #[default]
    Unknown,
}

impl fmt::Display for ApNoteType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
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

impl From<HashMap<String, String>> for Metadata {
    fn from(meta: HashMap<String, String>) -> Self {
        Metadata {
            twitter_title: meta.get("twitter:title").cloned(),
            description: meta.get("description").cloned(),
            og_description: meta.get("og:description").cloned(),
            og_title: meta.get("og:title").cloned(),
            og_image: meta.get("og:image").cloned(),
            og_site_name: meta.get("og:site_name").cloned(),
            twitter_image: meta.get("twitter:image").cloned(),
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
    pub attributed_to: String,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: ApNoteType,
    //pub to: Vec<String>,
    pub to: MaybeMultiple<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<String>>,
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
    pub ephemeral_announce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_actors: Option<Vec<ApActor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_liked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_targeted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_timestamp: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_metadata: Option<Vec<Metadata>>,
}

impl ApNote {
    pub fn to(mut self, to: String) -> Self {
        if let MaybeMultiple::Multiple(v) = self.to {
            let mut t = v;
            t.push(to);
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

    pub fn is_public(&self) -> bool {
        match self.to.clone() {
            MaybeMultiple::Multiple(n) => {
                n.contains(&"https://www.w3.org/ns/activitystreams#Public".to_string())
            }
            MaybeMultiple::Single(n) => n == *"https://www.w3.org/ns/activitystreams#Public",
        }
    }
}

impl Default for ApNote {
    fn default() -> ApNote {
        ApNote {
            context: Some(ApContext::default()),
            tag: None,
            attributed_to: String::new(),
            id: None,
            kind: ApNoteType::Note,
            to: MaybeMultiple::Multiple(vec![]),
            url: None,
            published: None,
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
            ephemeral_announce: None,
            ephemeral_actors: None,
            ephemeral_liked: None,
            ephemeral_targeted: None,
            ephemeral_timestamp: None,
            ephemeral_metadata: None,
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
                    get_ap_id_from_username(profile.clone().username)
                } else {
                    vault.clone().remote_actor
                }
            },
            to: {
                if vault.outbound {
                    MaybeMultiple::Multiple(vec![vault.remote_actor])
                } else {
                    MaybeMultiple::Multiple(vec![get_ap_id_from_username(profile.username)])
                }
            },
            id: Some(format!(
                "https://{}/vault/{}",
                *crate::SERVER_NAME,
                vault.uuid
            )),
            content: vault.encrypted_data,
            published: Some(vault.created_at.to_rfc2822()),
            ..Default::default()
        }
    }
}

// we're pre-loading the ApActor objects here so that we don't have to make
// separate calls to retrieve that data at the client; making those extra calls
// is particularly problematic for unauthenticated retrieval as it would require
// that we expose the endpoint for retreiving RemoteActor data to the world
pub type QualifiedTimelineItem = (TimelineItem, Option<Vec<ApActor>>);

impl From<QualifiedTimelineItem> for ApNote {
    fn from((timeline, actors): QualifiedTimelineItem) -> Self {
        ApNote::from(((timeline, None, None), actors))
    }
}

pub type FullyQualifiedTimelineItem = (
    (TimelineItem, Option<Like>, Option<TimelineItemCc>),
    Option<Vec<ApActor>>,
);

impl From<FullyQualifiedTimelineItem> for ApNote {
    fn from(((timeline, like, cc), actors): FullyQualifiedTimelineItem) -> Self {
        ApNote {
            tag: {
                if let Some(x) = timeline.tag {
                    match serde_json::from_value(x) {
                        Ok(y) => y,
                        Err(_) => None,
                    }
                } else {
                    None
                }
            },
            attributed_to: timeline.attributed_to,
            id: Some(timeline.ap_id),
            url: timeline.url,
            published: timeline.published,
            replies: Option::None,
            in_reply_to: timeline.in_reply_to,
            content: timeline.content.unwrap_or_default(),
            summary: timeline.summary,
            sensitive: timeline.ap_sensitive,
            atom_uri: timeline.atom_uri,
            in_reply_to_atom_uri: timeline.in_reply_to_atom_uri,
            conversation: timeline.conversation,
            content_map: {
                if let Some(x) = timeline.content_map {
                    match serde_json::from_value(x) {
                        Ok(y) => y,
                        Err(_) => None,
                    }
                } else {
                    None
                }
            },
            attachment: {
                if let Some(x) = timeline.attachment {
                    match serde_json::from_value(x) {
                        Ok(y) => y,
                        Err(_) => None,
                    }
                } else {
                    None
                }
            },
            ephemeral_announce: timeline.announce,
            ephemeral_actors: actors,
            ephemeral_liked: Some(like.is_some()),
            ephemeral_targeted: Some(cc.is_some()),
            ephemeral_timestamp: Some(timeline.created_at),
            ephemeral_metadata: {
                if let Some(x) = timeline.metadata {
                    match serde_json::from_value(x) {
                        Ok(y) => y,
                        Err(_) => None,
                    }
                } else {
                    None
                }
            },
            ..Default::default()
        }
    }
}

impl From<ApActor> for ApNote {
    fn from(actor: ApActor) -> Self {
        ApNote {
            tag: Option::from(vec![]),
            attributed_to: actor.id.unwrap(),
            id: Option::None,
            kind: ApNoteType::Note,
            to: MaybeMultiple::Multiple(vec![]),
            content: String::new(),
            ..Default::default()
        }
    }
}

impl From<NewNote> for ApNote {
    fn from(note: NewNote) -> Self {
        let kind = match note.kind.as_str() {
            "Note" => ApNoteType::Note,
            "EncryptedNote" => ApNoteType::EncryptedNote,
            _ => ApNoteType::default(),
        };

        ApNote {
            tag: match serde_json::from_value(note.tag.into()) {
                Ok(x) => x,
                Err(_) => Option::None,
            },
            attributed_to: note.attributed_to,
            published: Option::from(Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
            id: Option::from(format!(
                "https://{}/notes/{}",
                *crate::SERVER_NAME,
                note.uuid
            )),
            kind,
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
            ..Default::default()
        }
    }
}

impl From<Note> for ApNote {
    fn from(note: Note) -> Self {
        let kind = match note.kind.as_str() {
            "Note" => ApNoteType::Note,
            "EncryptedNote" => ApNoteType::EncryptedNote,
            _ => ApNoteType::default(),
        };

        ApNote {
            tag: match serde_json::from_value(note.tag.into()) {
                Ok(x) => x,
                Err(_) => Option::None,
            },
            attributed_to: note.attributed_to,
            published: Option::from(Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
            id: Option::from(format!(
                "https://{}/notes/{}",
                *crate::SERVER_NAME,
                note.uuid
            )),
            kind,
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
            ..Default::default()
        }
    }
}

type RemoteNoteAndMetadata = (RemoteNote, Option<Vec<Metadata>>);

impl From<RemoteNote> for ApNote {
    fn from(remote_note: RemoteNote) -> Self {
        (remote_note, None).into()
    }
}

impl From<RemoteNoteAndMetadata> for ApNote {
    fn from((remote_note, metadata): RemoteNoteAndMetadata) -> ApNote {
        let kind = match remote_note.kind.as_str() {
            "Note" => ApNoteType::Note,
            "EncryptedNote" => ApNoteType::EncryptedNote,
            _ => ApNoteType::default(),
        };

        ApNote {
            id: Some(remote_note.ap_id),
            kind,
            published: remote_note.published,
            url: remote_note.url,
            to: match serde_json::from_value(remote_note.ap_to.into()) {
                Ok(x) => x,
                Err(_) => MaybeMultiple::Multiple(vec![]),
            },
            cc: match serde_json::from_value(remote_note.cc.into()) {
                Ok(x) => x,
                Err(_) => None,
            },
            tag: match serde_json::from_value(remote_note.tag.into()) {
                Ok(x) => x,
                Err(_) => None,
            },
            attributed_to: remote_note.attributed_to,
            content: remote_note.content,
            replies: match serde_json::from_value(remote_note.replies.into()) {
                Ok(x) => x,
                Err(_) => None,
            },
            in_reply_to: remote_note.in_reply_to,
            attachment: match serde_json::from_value(remote_note.attachment.unwrap_or_default()) {
                Ok(x) => x,
                Err(_) => None,
            },
            conversation: remote_note.conversation,
            ephemeral_timestamp: Some(remote_note.created_at),
            ephemeral_metadata: metadata,
            ..Default::default()
        }
    }
}
