use crate::activity_pub::{ApActor, ApCollection, ApInstrument, ApNote, Outbox};
use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::fairings::faktory::FaktoryConnection;
use crate::models::profiles::Profile;
use crate::{Identifier, MaybeMultiple};

use enum_dispatch::enum_dispatch;
use rocket::http::{ContentType, Status};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use super::collection::ApCollectionPage;
use super::delete::ApTombstone;
use super::session::ApSession;

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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[enum_dispatch]
#[serde(untagged)]
pub enum ApObject {
    Tombstone(ApTombstone),
    Session(ApSession),
    Instrument(ApInstrument),
    Note(ApNote),
    Actor(ApActor),
    Collection(ApCollection),
    CollectionPage(ApCollectionPage),

    // These members exist to catch unknown object types
    Plain(String),
    Identifier(Identifier),
    Basic(ApBasicContent),
    Complex(MaybeMultiple<Value>),
}

impl Outbox for String {
    async fn outbox(
        &self,
        _conn: Db,
        _faktory: FaktoryConnection,
        _events: EventChannels,
        _profile: Profile,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl Outbox for Identifier {
    async fn outbox(
        &self,
        _conn: Db,
        _faktory: FaktoryConnection,
        _events: EventChannels,
        _profile: Profile,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl Outbox for MaybeMultiple<Value> {
    async fn outbox(
        &self,
        _conn: Db,
        _faktory: FaktoryConnection,
        _events: EventChannels,
        _profile: Profile,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl Outbox for ApBasicContent {
    async fn outbox(
        &self,
        _conn: Db,
        _faktory: FaktoryConnection,
        _events: EventChannels,
        _profile: Profile,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
}

// impl Outbox for ApObject {
//     async fn outbox(
//         &self,
//         conn: Db,
//         faktory: FaktoryConnection,
//         events: EventChannels,
//         profile: Profile,
//     ) -> Result<String, Status> {
//         match self {
//             ApObject::Note(object) => object.outbox(conn, faktory, events, profile).await,
//             ApObject::Session(object) => object.outbox(conn, faktory, events, profile).await,
//             _ => Err(Status::NoContent),
//         }
//     }
// }

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
    pub kind: ApHashtagType,
    pub name: String,
    pub href: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApEmoji {
    #[serde(rename = "type")]
    pub kind: ApEmojiType,
    pub id: String,
    pub name: String,
    pub updated: Option<String>,
    pub icon: ApImage,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ApTag {
    Emoji(ApEmoji),
    Mention(ApMention),
    HashTag(ApHashtag),
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

fn get_media_type(url: &str) -> Option<String> {
    if let Ok(url) = Url::parse(url) {
        if let Some(ext) = url.path().split('.').last() {
            ContentType::from_extension(ext).map(|x| x.to_string())
        } else {
            None
        }
    } else {
        None
    }
}

impl From<String> for ApImage {
    fn from(url: String) -> Self {
        ApImage {
            kind: ApImageType::Image,
            media_type: get_media_type(&url),
            url,
        }
    }
}
