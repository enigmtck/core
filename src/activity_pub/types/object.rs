use crate::activity_pub::{ApActor, ApCollection, ApInstrument, ApNote};
use crate::{Identifier, MaybeMultiple};
use image::io::Reader;
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::fmt::Debug;

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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
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

// {
//     match format {
//         ImageFormat::Png => Some("image/png".to_string()),
//         ImageFormat::Gif => Some("image/gif".to_string()),
//         ImageFormat::Jpeg => Some("image/jpg".to_string()),
//         ImageFormat::WebP => Some("image/webp".to_string()),
//         ImageFormat::Pnm => Some("image/pnm".to_string()),
//         ImageFormat::Tiff => Some("image/tiff".to_string()),
//         ImageFormat::Tga => Some("image/tga".to_string()),
//         ImageFormat::Dds => Some("image/dds".to_string()),
//         ImageFormat::Bmp => Some("image/bmp".to_string()),
//         ImageFormat::Ico => Some("image/ico".to_string()),
//         _ => None,
//     }
// }
impl TryFrom<String> for ApAttachment {
    type Error = &'static str;

    fn try_from(filename: String) -> Result<Self, Self::Error> {
        let path = &format!("{}/uploads/{}", *crate::MEDIA_DIR, filename);

        if let Ok(meta) = rexiv2::Metadata::new_from_path(path) {
            meta.clear();
            meta.save_to_file(path).ok();
        }

        if let Ok(img) = Reader::open(path) {
            if let Ok(img) = img.with_guessed_format() {
                if let Some(_format) = img.format() {
                    if let Ok(decode) = img.decode() {
                        let decode =
                            decode.resize(1024, 768, image::imageops::FilterType::Gaussian);
                        decode.save_with_format(path, ImageFormat::Png).ok();
                        let blurhash = blurhash::encode(
                            4,
                            3,
                            decode.width(),
                            decode.height(),
                            &decode.to_rgba8().into_vec(),
                        );
                        Ok(ApAttachment {
                            kind: ApAttachmentType::Document,
                            name: None,
                            summary: None,
                            media_type: Some("image/png".to_string()),
                            url: Some(format!("{}/media/uploads/{}", *crate::SERVER_URL, filename)),
                            blurhash: Some(blurhash),
                            width: Some(decode.width() as i32),
                            height: Some(decode.height() as i32),
                            signature_algorithm: None,
                            signature_value: None,
                            href: None,
                        })
                    } else {
                        log::error!("FAILED TO DECODE IMAGE");
                        Err("FAILED TO DECODE IMAGE")
                    }
                } else {
                    log::error!("FAILED TO DETERMINE FORMAT");
                    Err("FAILED TO DETERMINE FORMAT")
                }
            } else {
                log::error!("FAILED TO GUESS FORMAT");
                Err("FAILED TO GUESS FORMAT")
            }
        } else {
            log::error!("FAILED TO OPEN FILE");
            Err("FAILED TO OPEN FILE")
        }
    }
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
