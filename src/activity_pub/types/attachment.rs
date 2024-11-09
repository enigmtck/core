use std::fmt;
use std::fmt::Debug;

use anyhow::anyhow;
use image::io::Reader;
use image::ImageFormat;
use serde::{Deserialize, Serialize};

use crate::OrdValue;

use super::object::ApImage;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum ApAttachment {
    Document(ApDocument),
    Image(ApImage),
    PropertyValue(ApPropertyValue),
    Link(ApLink),
    Uncategorized(OrdValue),
}

impl TryFrom<String> for ApAttachment {
    type Error = anyhow::Error;

    fn try_from(filename: String) -> Result<Self, Self::Error> {
        let path = &format!("{}/uploads/{}", *crate::MEDIA_DIR, filename);

        if let Ok(meta) = rexiv2::Metadata::new_from_path(path) {
            meta.clear();
            meta.save_to_file(path).ok();
        }

        let img = Reader::open(path).map_err(anyhow::Error::msg)?;
        let img = img.with_guessed_format().map_err(anyhow::Error::msg)?;
        let _format = img
            .format()
            .ok_or_else(|| anyhow!("FAILED TO DETERMINE FORMAT"))?;
        let decode = img.decode().map_err(anyhow::Error::msg)?;
        let decode = decode.resize(1024, 768, image::imageops::FilterType::Gaussian);
        decode.save_with_format(path, ImageFormat::Png).ok();
        let blurhash = blurhash::encode(
            4,
            3,
            decode.width(),
            decode.height(),
            &decode.to_rgba8().into_vec(),
        )
        .map_err(anyhow::Error::msg)?;
        Ok(ApAttachment::Document(ApDocument {
            kind: ApDocumentType::Document,
            media_type: Some("image/png".to_string()),
            url: Some(format!("{}/media/uploads/{}", *crate::SERVER_URL, filename)),
            blurhash: Some(blurhash),
            width: Some(decode.width() as i32),
            height: Some(decode.height() as i32),
        }))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ApDocumentType {
    #[serde(alias = "document")]
    Document,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct ApDocument {
    #[serde(rename = "type")]
    pub kind: ApDocumentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blurhash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ApPropertyValueType {
    PropertyValue,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct ApPropertyValue {
    #[serde(rename = "type")]
    pub kind: ApPropertyValueType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ApLinkType {
    #[serde(alias = "link")]
    Link,
}

impl fmt::Display for ApLinkType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct ApLink {
    #[serde(rename = "type")]
    pub kind: ApLinkType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rel: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct ApProof {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub created: Option<String>,
    pub proof_purpose: Option<String>,
    pub proof_value: Option<String>,
    pub verification_method: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ApVerifiableIdentityStatementType {
    VerifiableIdentityStatement,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct ApVerifiableIdentityStatement {
    #[serde(rename = "type")]
    pub kind: ApVerifiableIdentityStatementType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<ApProof>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub also_known_as: Option<String>,
}
