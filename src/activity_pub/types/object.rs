use crate::activity_pub::{ApActivity, ApActor, ApCollection, ApInstrument, ApNote};
use crate::MaybeMultiple;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::fmt::Debug;

use super::session::ApSession;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApSignatureType {
    RsaSignature2017,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApSignature {
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<ApSignatureType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    creator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ApContext {
    Plain(String),
    Complex(Vec<Value>),
}

impl Default for ApContext {
    fn default() -> Self {
        ApContext::Plain("https://www.w3.org/ns/activitystreams".to_string())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApIdentifier {
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ApBasicContentType {
    IdentityKey,
    SessionKey,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApBasicContent {
    #[serde(rename = "type")]
    pub kind: ApBasicContentType,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ApTombstoneType {
    Tombstone,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApTombstone {
    #[serde(rename = "type")]
    pub kind: ApTombstoneType,
    pub id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(untagged)]
pub enum ApObject {
    Plain(String),
    Tombstone(ApTombstone),
    Session(ApSession),
    Instrument(ApInstrument),
    Note(ApNote),
    Actor(ApActor),
    Collection(ApCollection),
    Identifier(ApIdentifier),
    Basic(ApBasicContent),
    Complex(MaybeMultiple<Value>),
    #[default]
    Unknown,
}

impl ApObject {
    pub fn is_plain(&self) -> bool {
        matches!(*self, ApObject::Plain(_))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApMentionType {
    Mention,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApHashtagType {
    Hashtag,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApEmojiType {
    Emoji,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApMention {
    #[serde(rename = "type")]
    pub kind: ApMentionType,
    pub name: String,
    pub href: Option<String>,
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApHashtag {
    #[serde(rename = "type")]
    kind: ApHashtagType,
    name: String,
    href: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApEmoji {
    #[serde(rename = "type")]
    kind: ApEmojiType,
    id: String,
    name: String,
    updated: Option<String>,
    icon: ApImage,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ApTag {
    Emoji(ApEmoji),
    Mention(ApMention),
    HashTag(ApHashtag),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApAttachmentType {
    PropertyValue,
    Document,
    IdentityProof,
    Link,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApAttachment {
    #[serde(rename = "type")]
    pub kind: ApAttachmentType,
    pub name: Option<String>,
    pub summary: Option<String>,
    pub media_type: Option<String>,
    pub url: Option<String>,
    pub blurhash: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub signature_algorithm: Option<String>,
    pub signature_value: Option<String>,
    pub href: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApProof {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub created: Option<String>,
    pub proof_purpose: Option<String>,
    pub proof_value: Option<String>,
    pub verification_method: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApEndpoint {
    pub shared_inbox: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApImageType {
    Image,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApImage {
    #[serde(rename = "type")]
    pub kind: ApImageType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApLinkType {
    Mention,
}

impl fmt::Display for ApLinkType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, PartialEq, Eq, Deserialize, Clone, Debug, Default)]
pub enum ApActivityType {
    Create,
    Update,
    Delete,
    Follow,
    Accept,
    Reject,
    Add,
    Remove,
    Like,
    Announce,
    Undo,
    Invite,
    Join,
    #[default]
    Unknown,
}

impl fmt::Display for ApActivityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl From<String> for ApActivityType {
    fn from(data: String) -> Self {
        data.as_str().into()
    }
}

impl From<&str> for ApActivityType {
    fn from(data: &str) -> Self {
        match data {
            "Create" => ApActivityType::Create,
            "Update" => ApActivityType::Update,
            "Delete" => ApActivityType::Delete,
            "Follow" => ApActivityType::Follow,
            "Accept" => ApActivityType::Accept,
            "Reject" => ApActivityType::Reject,
            "Add" => ApActivityType::Add,
            "Remove" => ApActivityType::Remove,
            "Like" => ApActivityType::Like,
            "Announce" => ApActivityType::Announce,
            "Undo" => ApActivityType::Undo,
            "Invite" => ApActivityType::Invite,
            "Join" => ApActivityType::Join,
            _ => ApActivityType::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ApBaseObjectSuper {
    Activity(ApActivity),
    Actor(ApActor),
    Object(ApObject),
}
