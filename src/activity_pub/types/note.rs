use core::fmt;
use std::{collections::HashMap, fmt::Debug};

use crate::{
    activity_pub::{
        ApActor, ApAttachment, ApContext, ApFlexible, ApFlexibleString, ApInstruments, ApTag,
    },
    helper::get_ap_id_from_username,
    models::{
        notes::{NewNote, Note},
        profiles::Profile,
        remote_notes::RemoteNote,
        timeline::TimelineItem,
        vault::VaultItem,
    },
};
use chrono::Utc;
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
    pub to: ApFlexibleString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replies: Option<ApFlexible>,
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
}

impl ApNote {
    pub fn to(mut self, to: String) -> Self {
        if let ApFlexibleString::Multiple(v) = self.to {
            let mut t = v;
            t.push(to);
            self.to = ApFlexibleString::Multiple(t);
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
            ApFlexibleString::Multiple(n) => {
                n.contains(&"https://www.w3.org/ns/activitystreams#Public".to_string())
            }
            ApFlexibleString::Single(n) => n == *"https://www.w3.org/ns/activitystreams#Public",
        }
    }
}

impl Default for ApNote {
    fn default() -> ApNote {
        ApNote {
            context: Option::from(ApContext::Plain(
                "https://www.w3.org/ns/activitystreams".to_string(),
            )),
            tag: Option::None,
            attributed_to: String::new(),
            id: Option::None,
            kind: ApNoteType::Note,
            to: ApFlexibleString::Multiple(vec![]),
            url: Option::None,
            published: Option::None,
            cc: Option::None,
            replies: Option::None,
            attachment: Option::None,
            in_reply_to: Option::None,
            content: String::new(),
            summary: Option::None,
            sensitive: Option::None,
            atom_uri: Option::None,
            in_reply_to_atom_uri: Option::None,
            conversation: Option::None,
            content_map: Option::None,
            instrument: Option::None,
            ephemeral_announce: Option::None,
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
                    ApFlexibleString::Multiple(vec![vault.remote_actor])
                } else {
                    ApFlexibleString::Multiple(vec![get_ap_id_from_username(profile.username)])
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

impl From<TimelineItem> for ApNote {
    fn from(timeline: TimelineItem) -> Self {
        ApNote {
            tag: match serde_json::from_value(timeline.tag.unwrap_or_default()) {
                Ok(x) => x,
                Err(_) => Option::None,
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
            content_map: match serde_json::from_value(timeline.content_map.unwrap_or_default()) {
                Ok(x) => x,
                Err(_) => Option::None,
            },
            attachment: match serde_json::from_value(timeline.attachment.unwrap_or_default()) {
                Ok(x) => x,
                Err(_) => Option::None,
            },
            ephemeral_announce: timeline.announce,
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
            to: ApFlexibleString::Multiple(vec![]),
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
                Err(_) => ApFlexibleString::Multiple(vec![]),
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
                Err(_) => ApFlexibleString::Multiple(vec![]),
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

impl From<RemoteNote> for ApNote {
    fn from(remote_note: RemoteNote) -> ApNote {
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
                Err(_) => ApFlexibleString::Multiple(vec![]),
            },
            cc: match serde_json::from_value(remote_note.cc.into()) {
                Ok(x) => x,
                Err(_) => Option::None,
            },
            tag: match serde_json::from_value(remote_note.tag.into()) {
                Ok(x) => x,
                Err(_) => Option::None,
            },
            attributed_to: remote_note.attributed_to,
            content: remote_note.content,
            replies: match serde_json::from_value(remote_note.replies.into()) {
                Ok(x) => x,
                Err(_) => Option::None,
            },
            in_reply_to: remote_note.in_reply_to,
            attachment: match serde_json::from_value(remote_note.attachment.unwrap_or_default()) {
                Ok(x) => x,
                Err(_) => Option::None,
            },
            conversation: remote_note.conversation,
            ..Default::default()
        }
    }
}
