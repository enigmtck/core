use crate::activity_pub::{ApActivity, ApActor, ApCollection, ApNote, ApOrderedCollection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::fmt::Debug;

// #[derive(Serialize, Deserialize, Clone, Debug)]
// #[serde(rename_all = "camelCase")]
// pub struct ApMastodonContextBasic {
//     #[serde(rename = "@id")]
//     id: String,
//     #[serde(rename = "@type")]
//     #[serde(skip_serializing_if = "Option::is_none")]
//     kind: Option<String>,
//     #[serde(rename = "@container")]
//     #[serde(skip_serializing_if = "Option::is_none")]
//     container: Option<String>,
// }

// #[derive(Serialize, Deserialize, Clone, Debug)]
// #[serde(rename_all = "camelCase")]
// pub struct ApMastodonContext {
//     #[serde(skip_serializing_if = "Option::is_none")]
//     manually_approves_followers: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     toot: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     featured: Option<ApMastodonContextBasic>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     featured_tags: Option<ApMastodonContextBasic>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     also_known_as: Option<ApMastodonContextBasic>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     moved_to: Option<ApMastodonContextBasic>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     schema: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     value: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     discoverable: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     claim: Option<ApMastodonContextBasic>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     fingerprint_key: Option<ApMastodonContextBasic>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     identity_key: Option<ApMastodonContextBasic>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     devices: Option<ApMastodonContextBasic>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     message_franking: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     message_type: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     cipher_text: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     suspended: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     focal_point: Option<ApMastodonContextBasic>,
// }

// #[derive(Serialize, Deserialize, Clone, Debug)]
// #[serde(rename_all = "camelCase")]
// pub struct ApMastodonObject {
//     #[serde(skip_serializing_if = "Option::is_none")]
//     id: Option<String>,
// }

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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(untagged)]
pub enum ApObject {
    Plain(String),
    Note(ApNote),
    Actor(ApActor),
    OrderedCollection(ApOrderedCollection),
    Collection(ApCollection),
    Identifier(ApIdentifier),
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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApTag {
    #[serde(rename = "type")]
    pub kind: ApTagType,
    pub name: String,
    pub href: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ApFlexible {
    Single(Value),
    Multiple(Vec<Value>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApBaseObjectType {
    Object,
    Link,
    Activity,
    IntransitiveActivity,
    Collection,
    OrderedCollection,
    CollectionPage,
    OrderedCollectionPage,
}

impl fmt::Display for ApBaseObjectType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApObjectType {
    Article,
    Audio,
    Document,
    Event,
    Image,
    Note,
    Page,
    Place,
    Profile,
    Relationship,
    Tombstone,
    Video,
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
    Base(ApBaseObject),
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApBaseObject {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bcc: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<Vec<ApTag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment: Option<ApFlexible>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributed_to: Option<ApFlexible>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<ApFlexible>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator: Option<ApFlexible>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<ApFlexible>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<ApFlexible>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<ApFlexible>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<ApFlexible>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replies: Option<ApFlexible>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<ApFlexible>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bto: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub kind: Option<ApBaseObjectType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing)]
    pub uuid: Option<String>,
}
