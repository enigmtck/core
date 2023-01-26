use crate::{
    activity_pub::{ApActor, ApAttachment, ApContext, ApFlexible, ApObjectType, ApTag},
    models::{notes::NewNote, remote_notes::RemoteNote, timeline::TimelineItem},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub kind: ApObjectType,
    pub to: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<ApFlexible>,
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
    pub content_map: Option<Value>,
}

impl ApNote {
    pub fn to(mut self, to: String) -> Self {
        self.to.push(to);
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

impl Default for ApNote {
    fn default() -> ApNote {
        ApNote {
            context: Option::from(ApContext::Plain(
                "https://www.w3.org/ns/activitystreams".to_string(),
            )),
            tag: Option::None,
            attributed_to: String::new(),
            id: Option::None,
            kind: ApObjectType::Note,
            to: vec![],
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
        }
    }
}

impl From<TimelineItem> for ApNote {
    fn from(timeline: TimelineItem) -> Self {
        let url: Option<ApFlexible> = {
            if let Some(url) = timeline.url {
                Option::from(ApFlexible::from(url))
            } else {
                Option::None
            }
        };

        ApNote {
            tag: serde_json::from_value(timeline.tag.unwrap_or_default()).unwrap(),
            attributed_to: timeline.attributed_to,
            id: Some(timeline.ap_id),
            url,
            published: timeline.published,
            replies: Option::None,
            in_reply_to: timeline.in_reply_to,
            content: timeline.content,
            summary: timeline.summary,
            sensitive: timeline.ap_sensitive,
            atom_uri: timeline.atom_uri,
            in_reply_to_atom_uri: timeline.in_reply_to_atom_uri,
            conversation: timeline.conversation,
            content_map: timeline.content_map,
            attachment: serde_json::from_value(timeline.attachment.unwrap_or_default()).unwrap(),
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
            kind: ApObjectType::Note,
            to: vec![],
            content: String::new(),
            ..Default::default()
        }
    }
}

impl From<NewNote> for ApNote {
    fn from(note: NewNote) -> Self {
        let kind = match note.kind.as_str() {
            "Note" => ApObjectType::Note,
            "EncryptedNote" => ApObjectType::EncryptedNote,
            _ => ApObjectType::default(),
        };

        ApNote {
            tag: serde_json::from_value(note.tag.into()).unwrap(),
            attributed_to: note.attributed_to,
            published: Option::from(Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
            id: Option::from(format!(
                "https://{}/notes/{}",
                *crate::SERVER_NAME,
                note.uuid
            )),
            kind,
            to: serde_json::from_value(note.ap_to).unwrap(),
            content: note.content,
            cc: serde_json::from_value(note.cc.into()).unwrap(),
            ..Default::default()
        }
    }
}

impl From<RemoteNote> for ApNote {
    fn from(remote_note: RemoteNote) -> ApNote {
        let kind = match remote_note.kind.as_str() {
            "Note" => ApObjectType::Note,
            "EncryptedNote" => ApObjectType::EncryptedNote,
            _ => ApObjectType::default(),
        };

        ApNote {
            id: Some(remote_note.ap_id),
            kind,
            published: remote_note.published,
            url: Option::from(ApFlexible::Single(remote_note.url.into())),
            to: serde_json::from_value(remote_note.ap_to.into()).unwrap(),
            cc: serde_json::from_value(remote_note.cc.into()).unwrap(),
            tag: serde_json::from_value(remote_note.tag.into()).unwrap(),
            attributed_to: remote_note.attributed_to,
            content: remote_note.content,
            replies: serde_json::from_value(remote_note.replies.into()).unwrap(),
            in_reply_to: remote_note.in_reply_to,
            ..Default::default()
        }
    }
}
