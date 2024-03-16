use std::fmt;
use std::fmt::Debug;

use image::io::Reader;
use image::ImageFormat;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum ApAttachment {
    Document(ApDocument),
    PropertyValue(ApPropertyValue),
    Link(ApLink),
    Proof(ApProof),
    VerifiableIdentityStatement(ApVerifiableIdentityStatement),
}

impl TryFrom<String> for ApAttachment {
    type Error = String;

    fn try_from(filename: String) -> Result<Self, Self::Error> {
        let path = &format!("{}/uploads/{}", *crate::MEDIA_DIR, filename);

        if let Ok(meta) = rexiv2::Metadata::new_from_path(path) {
            meta.clear();
            meta.save_to_file(path).ok();
        }

        let img = Reader::open(path).map_err(|_| "FAILED TO OPEN FILE")?;
        let img = img
            .with_guessed_format()
            .map_err(|_| "FAILED TO GUESS FORMAT")?;
        let _format = img.format().ok_or("FAILED TO DETERMINE FORMAT")?;
        let decode = img.decode().map_err(|_| "FAILED TO DECODE IMAGE")?;
        let decode = decode.resize(1024, 768, image::imageops::FilterType::Gaussian);
        decode.save_with_format(path, ImageFormat::Png).ok();
        let blurhash = blurhash::encode(
            4,
            3,
            decode.width(),
            decode.height(),
            &decode.to_rgba8().into_vec(),
        )
        .map_err(|e| e.to_string())?;
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApDocumentType {
    #[serde(alias = "document")]
    Document
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApDocument {
    #[serde(rename = "type")]
    pub kind: ApDocumentType,
    pub media_type: Option<String>,
    pub url: Option<String>,
    pub blurhash: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApPropertyValueType {
    PropertyValue,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApPropertyValue {
    #[serde(rename = "type")]
    pub kind: ApPropertyValueType,
    pub name: Option<String>,
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApLinkType {
    #[serde(alias = "link")]
    Link,
}

impl fmt::Display for ApLinkType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApLink {
    #[serde(rename = "type")]
    pub kind: ApLinkType,
    pub href: Option<String>,
    pub media_type: Option<String>,
    pub name: Option<String>,
    pub rel: Option<Vec<String>>,
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
pub enum ApVerifiableIdentityStatementType {
    VerifiableIdentityStatement,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApVerifiableIdentityStatement {
    #[serde(rename = "type")]
    pub kind: ApVerifiableIdentityStatementType,
    pub subject: Option<String>,
    pub proof: Option<ApProof>,
    pub also_known_as: Option<String>,
}
