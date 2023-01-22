use crate::activity_pub::{ApActivity, ApActor, ApCollection, ApNote, ApOrderedCollection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::fmt::Debug;

use super::session::ApSession;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApSignature {
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    creator: Option<String>,
    created: Option<String>,
    signature_value: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(untagged)]
pub enum ApContext {
    Plain(String),
    Complex(Vec<Value>),
    #[default]
    Unknown,
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(untagged)]
pub enum ApObject {
    Plain(String),
    Session(ApSession),
    Note(ApNote),
    Actor(ApActor),
    OrderedCollection(ApOrderedCollection),
    Collection(ApCollection),
    Identifier(ApIdentifier),
    Basic(ApBasicContent),
    Complex(ApFlexible),
    #[default]
    Unknown,
}

impl ApObject {
    pub fn is_plain(&self) -> bool {
        matches!(*self, ApObject::Plain(_))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApTagType {
    Mention,
    Hashtag,
    Emoji,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApTag {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: ApTagType,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<ApImage>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApAttachmentType {
    PropertyValue,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApAttachment {
    #[serde(rename = "type")]
    pub kind: ApAttachmentType,
    pub name: String,
    pub value: String,
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
#[serde(untagged)]
pub enum ApFlexible {
    Single(Value),
    Multiple(Vec<Value>),
}

impl From<String> for ApFlexible {
    fn from(data: String) -> Self {
        ApFlexible::Single(Value::from(data))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(untagged)]
pub enum ApInstrument {
    Multiple(Vec<ApObject>),
    Single(Box<ApObject>),
    #[default]
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApBaseObjectType {
    Object,
    Link,
    Activity,
    IntransitiveActivity,
    Collection,
    OrderedCollection,
    CollectionPage,
    OrderedCollectionPage,
    #[default]
    Unknown,
}

impl fmt::Display for ApBaseObjectType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApObjectType {
    Article,
    Document,
    Image,
    Note,
    Page,
    Profile,
    EncryptedSession,
    IdentityKey,
    SessionKey,
    EncryptedNote,
    #[default]
    Unknown,
}

impl fmt::Display for ApObjectType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
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

#[derive(Serialize, PartialEq, Eq, Deserialize, Clone, Debug, Default)]
pub enum ApActorType {
    Application,
    Group,
    Organization,
    Person,
    Service,
    #[default]
    Unknown,
}

impl fmt::Display for ApActorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ApBaseObjectSuper {
    Activity(ApActivity),
    Actor(ApActor),
    Object(ApObject),
    //    Base(ApBaseObject),
}

// #[derive(Serialize, Deserialize, Clone, Debug)]
// #[serde(rename_all = "camelCase")]
// pub struct ApBaseObject {
//     #[serde(rename = "@context")]
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub context: Option<ApContext>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub to: Option<Vec<String>>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub cc: Option<Vec<String>>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub bcc: Option<Vec<String>>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub tag: Option<Vec<ApTag>>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub attachment: Option<ApFlexible>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub attributed_to: Option<ApFlexible>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub audience: Option<ApFlexible>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub content: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub name: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub end_time: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub generator: Option<ApFlexible>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub icon: Option<ApFlexible>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub in_reply_to: Option<ApFlexible>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub location: Option<ApFlexible>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub preview: Option<ApFlexible>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub published: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub replies: Option<ApFlexible>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub start_time: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub summary: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub updated: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub url: Option<ApFlexible>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub bto: Option<Vec<String>>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub media_type: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub duration: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     #[serde(rename = "type")]
//     pub kind: Option<ApBaseObjectType>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub id: Option<String>,

//     // Non-standard attributes
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub reference: Option<String>,
//     #[serde(skip_serializing)]
//     pub uuid: Option<String>,
// }

// impl Default for ApBaseObject {
//     fn default() -> ApBaseObject {
//         ApBaseObject {
//             context: Option::from(ApContext::Plain(
//                 "https://www.w3.org/ns/activitystreams".to_string(),
//             )),
//             to: Option::None,
//             cc: Option::None,
//             bcc: Option::None,
//             tag: Option::None,
//             attachment: Option::None,
//             attributed_to: Option::None,
//             audience: Option::None,
//             content: Option::None,
//             name: Option::None,
//             end_time: Option::None,
//             generator: Option::None,
//             icon: Option::None,
//             in_reply_to: Option::None,
//             location: Option::None,
//             preview: Option::None,
//             published: Option::None,
//             replies: Option::None,
//             start_time: Option::None,
//             summary: Option::None,
//             updated: Option::None,
//             url: Option::None,
//             bto: Option::None,
//             media_type: Option::None,
//             duration: Option::None,
//             kind: Option::None,
//             id: Option::None,
//             reference: Option::None,
//             uuid: Option::None,
//         }
//     }
// }
