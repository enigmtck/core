use crate::activity_pub::{retriever, ApActor, ApCollection, ApInstrument, ApNote, Outbox};
use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::models::actors::Actor;
use crate::models::cache::Cache;
use crate::models::objects::Object;
use crate::models::objects::ObjectType;
use crate::routes::ActivityJson;
use crate::{Identifier, MaybeMultiple, OrdValue, IMAGE_MEDIA_RE};

use anyhow::{anyhow, Error, Result};
use chrono::{DateTime, Utc};
use enum_dispatch::enum_dispatch;
use rocket::http::{ContentType, Status};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use url::Url;

use super::activity::ApActivity;
use super::attachment::ApDocument;
use super::delete::ApTombstone;
use super::question::ApQuestion;
use super::session::ApSession;
use super::Ephemeral;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum ApContext {
    Plain(String),
    Complex(Vec<OrdValue>),
}

impl Default for ApContext {
    fn default() -> Self {
        ApContext::activity_streams()
    }
}

impl ApContext {
    pub fn activity_streams() -> Self {
        ApContext::Plain("https://www.w3.org/ns/activitystreams".to_string())
    }

    pub fn full() -> Self {
        let context = json!(["https://www.w3.org/ns/activitystreams", "https://w3id.org/security/v1", {"manuallyApprovesFollowers": "as:manuallyApprovesFollowers", "toot": "http://joinmastodon.org/ns#", "alsoKnownAs": {"@id": "as:alsoKnownAs","@type": "@id"}, "discoverable": "toot:discoverable", "schema": "http://schema.org#", "PropertyValue": "schema:PropertyValue", "value": "schema:value"}]);
        ApContext::Complex(vec![OrdValue(context)])
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
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ApTimelineObject {
    Note(ApNote),
    Question(ApQuestion),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[enum_dispatch]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ApObject {
    Tombstone(ApTombstone),
    Session(ApSession),
    Instrument(ApInstrument),
    Note(ApNote),
    Question(ApQuestion),
    Actor(ApActor),
    Collection(ApCollection),

    // These members exist to catch unknown object types
    Identifier(Identifier),
    Basic(ApBasicContent),

    // In review, I suspect these two may overlap; however,
    // Plain is used as in an assignment in Collection
    Complex(MaybeMultiple<Value>),
    Plain(String),
}

impl FromIterator<ApInstrument> for Vec<ApObject> {
    fn from_iter<I: IntoIterator<Item = ApInstrument>>(iter: I) -> Self {
        iter.into_iter().map(ApObject::from).collect()
    }
}

impl TryFrom<Object> for ApObject {
    type Error = anyhow::Error;

    fn try_from(object: Object) -> Result<Self> {
        match object.as_type {
            ObjectType::Note => Ok(ApObject::Note(object.try_into()?)),
            _ => Err(anyhow!("unimplemented Object -> ApObject conversion")),
        }
    }
}

impl Outbox for String {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Actor,
        _raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl Outbox for Identifier {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Actor,
        _raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl Outbox for MaybeMultiple<Value> {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Actor,
        _raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl Outbox for ApBasicContent {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Actor,
        _raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl ApObject {
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            ApObject::Note(note) => {
                if let Some(ephemeral) = note.ephemeral.clone() {
                    ephemeral.updated_at.unwrap_or(Utc::now())
                } else {
                    Utc::now()
                }
            }
            ApObject::Question(question) => {
                if let Some(ephemeral) = question.ephemeral.clone() {
                    ephemeral.updated_at.unwrap_or(Utc::now())
                } else {
                    Utc::now()
                }
            }
            _ => Utc::now(),
        }
    }

    pub async fn load_ephemeral(&mut self, conn: &Db) -> Self {
        match self {
            ApObject::Note(ref mut note) => {
                if let Ok(actor) =
                    retriever::get_actor(conn, note.attributed_to.clone().to_string(), None, true)
                        .await
                {
                    note.ephemeral = Some(Ephemeral {
                        attributed_to: Some(vec![actor.into()]),
                        ..Default::default()
                    });
                }
                ApObject::Note(note.clone())
            }
            _ => self.clone(),
        }
    }
}

impl Cache for ApObject {
    async fn cache(&self, conn: &Db) -> &Self {
        match self {
            ApObject::Note(note) => {
                note.cache(conn).await;
            }
            ApObject::Question(question) => {
                question.cache(conn).await;
            }
            _ => (),
        }

        self
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum ApMentionType {
    Mention,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum ApHashtagType {
    Hashtag,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum ApEmojiType {
    Emoji,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub struct ApMention {
    #[serde(rename = "type")]
    pub kind: ApMentionType,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub struct ApHashtag {
    #[serde(rename = "type")]
    pub kind: ApHashtagType,
    pub name: String,
    pub href: String,
}

impl From<Object> for Vec<ApHashtag> {
    fn from(object: Object) -> Self {
        match ApObject::try_from(object) {
            Ok(ApObject::Note(note)) => note.into(),
            _ => vec![],
        }
    }
}

impl From<ApNote> for Vec<ApHashtag> {
    fn from(note: ApNote) -> Self {
        note.tag
            .unwrap_or_default()
            .iter()
            .filter_map(|tag| {
                if let ApTag::HashTag(tag) = tag {
                    Some(tag.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub struct ApEmoji {
    #[serde(rename = "type")]
    pub kind: ApEmojiType,
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    pub icon: ApImage,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[serde(untagged)]
pub enum ApTag {
    Emoji(ApEmoji),
    Mention(ApMention),
    HashTag(ApHashtag),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct ApEndpoint {
    pub shared_inbox: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum ApImageType {
    #[serde(alias = "image")]
    Image,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct ApImage {
    #[serde(rename = "type")]
    pub kind: ApImageType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    pub url: String,
}

fn get_media_type(url: &str) -> Option<String> {
    let url = Url::parse(url).ok()?;
    let ext = url.path().split('.').last()?;
    if ["png", "jpg", "jpeg", "gif", "bmp", "ico", "svg"].contains(&ext.to_lowercase().as_str()) {
        ContentType::from_extension(ext).map(|x| x.to_string())
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

impl TryFrom<ApDocument> for ApImage {
    type Error = Error;

    fn try_from(document: ApDocument) -> Result<Self, Self::Error> {
        let url = document.url.ok_or(Self::Error::msg("url is None"))?;
        let media_type = document
            .media_type
            .ok_or(Self::Error::msg("media_type is None"))?;

        IMAGE_MEDIA_RE
            .is_match(&media_type)
            .then_some(ApImage {
                kind: ApImageType::Image,
                media_type: Some(media_type),
                url,
            })
            .ok_or(Self::Error::msg("not cacheable"))
    }
}
