use image::io::Reader;
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Debug;

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
                        Ok(ApAttachment::Document(ApDocument {
                            kind: ApDocumentType::Document,
                            media_type: Some("image/png".to_string()),
                            url: Some(format!("{}/media/uploads/{}", *crate::SERVER_URL, filename)),
                            blurhash: Some(blurhash),
                            width: Some(decode.width() as i32),
                            height: Some(decode.height() as i32),
                        }))
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ApDocumentType {
    Document,
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
