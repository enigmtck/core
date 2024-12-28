use core::fmt;
use std::{collections::HashMap, fmt::Debug};

use crate::{
    activity_pub::{ApAttachment, ApContext, ApTag, Outbox},
    db::Db,
    fairings::events::EventChannels,
    models::{
        actors::Actor,
        cache::{cache_content, Cache},
        coalesced_activity::CoalescedActivity,
        from_serde,
        objects::Object,
    },
    routes::ActivityJson,
    MaybeMultiple,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    activity::ApActivity, actor::ApAddress, collection::ApCollectionType, note::ApNoteType,
    object::ApImage, Ephemeral,
};

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub enum ApQuestionType {
    #[default]
    #[serde(alias = "question")]
    Question,
}

impl fmt::Display for ApQuestionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl TryFrom<String> for ApQuestionType {
    type Error = &'static str;

    fn try_from(kind: String) -> Result<Self, Self::Error> {
        match kind.as_str() {
            "question" => Ok(ApQuestionType::Question),
            _ => Err("no match for {kind}"),
        }
    }
}

impl From<ApQuestionType> for String {
    fn from(kind: ApQuestionType) -> String {
        format!("{kind:#?}")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuestionCollection {
    total_items: i32,
    #[serde(rename = "type")]
    kind: ApCollectionType,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuestionNote {
    id: Option<String>,
    attributed_to: Option<String>,
    to: Option<MaybeMultiple<String>>,
    name: String,
    replies: Option<QuestionCollection>,
    #[serde(rename = "type")]
    kind: ApNoteType,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApQuestion {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApQuestionType,
    pub id: String,

    pub attributed_to: ApAddress,
    pub to: MaybeMultiple<ApAddress>,

    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub cc: MaybeMultiple<ApAddress>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<QuestionNote>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<QuestionNote>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_map: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voters_count: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<Vec<ApTag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment: Option<Vec<ApAttachment>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitive: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<Ephemeral>,
}

impl ApQuestion {
    pub fn dedup(mut self) -> Self {
        if let Some(mut ephemeral) = self.ephemeral {
            if let Some(mut announces) = ephemeral.announces {
                announces.sort();
                announces.dedup();
                ephemeral.announces = Some(announces);
            }

            if let Some(mut likes) = ephemeral.likes {
                likes.sort();
                likes.dedup();
                ephemeral.likes = Some(likes);
            }

            self.ephemeral = Some(ephemeral);
        }

        self
    }
}

impl Cache for ApQuestion {
    async fn cache(&self, conn: &Db) -> &Self {
        if let Some(attachments) = self.attachment.clone() {
            for attachment in attachments {
                cache_content(conn, attachment.clone().try_into()).await;
            }
        }

        if let Some(tags) = self.tag.clone() {
            for tag in tags {
                cache_content(conn, tag.clone().try_into()).await;
            }
        }

        if let Some(ephemeral) = self.ephemeral.clone() {
            if let Some(metadata_vec) = ephemeral.metadata.clone() {
                for metadata in metadata_vec {
                    if let Some(og_image) = metadata.og_image.clone() {
                        cache_content(conn, Ok(ApImage::from(og_image).into())).await;
                    }

                    if let Some(twitter_image) = metadata.twitter_image.clone() {
                        cache_content(conn, Ok(ApImage::from(twitter_image).into())).await;
                    }
                }
            }
        }

        self
    }
}

impl Outbox for ApQuestion {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Actor,
        _raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        Err(Status::NotImplemented)
    }
}

impl TryFrom<CoalescedActivity> for ApQuestion {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        let kind = coalesced
            .object_type
            .ok_or_else(|| anyhow::anyhow!("object_type is None"))?
            .to_string()
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to convert object_type: {}", e))?;

        let id = coalesced
            .object_as_id
            .ok_or_else(|| anyhow::anyhow!("object_id is None"))?;
        let url = coalesced
            .object_url
            .and_then(from_serde::<MaybeMultiple<String>>)
            .and_then(|x| x.single().ok());
        let to = coalesced
            .object_to
            .and_then(from_serde)
            .ok_or_else(|| anyhow::anyhow!("object_to is None"))?;
        let cc: MaybeMultiple<ApAddress> = coalesced.object_cc.into();
        let tag = coalesced.object_tag.and_then(from_serde);
        let attributed_to = coalesced
            .object_attributed_to
            .and_then(from_serde)
            .ok_or_else(|| anyhow::anyhow!("object_attributed_to is None"))?;
        let in_reply_to = coalesced.object_in_reply_to.and_then(from_serde);
        let content = coalesced.object_content;
        let conversation = coalesced.object_conversation;
        let attachment = coalesced.object_attachment.and_then(from_serde);
        let summary = coalesced.object_summary;
        let sensitive = coalesced.object_sensitive;
        let published = coalesced.object_published;
        let end_time = coalesced.object_end_time;
        let one_of = coalesced.object_one_of.and_then(from_serde);
        let any_of = coalesced.object_any_of.and_then(from_serde);
        let voters_count = coalesced.object_voters_count;

        Ok(ApQuestion {
            kind,
            id,
            url,
            to,
            cc,
            tag,
            attributed_to,
            in_reply_to,
            content,
            conversation,
            attachment,
            summary,
            sensitive,
            published,
            end_time,
            one_of,
            any_of,
            voters_count,
            ..Default::default()
        })
    }
}

impl TryFrom<Object> for ApQuestion {
    type Error = anyhow::Error;

    fn try_from(object: Object) -> Result<Self, Self::Error> {
        Ok(ApQuestion {
            id: object.as_id,
            attributed_to: from_serde(object.as_attributed_to.ok_or(anyhow!("no attributed_to"))?)
                .ok_or(anyhow!("failed to convert from Value"))?,
            to: from_serde(object.as_to.ok_or(anyhow!("as_to is None"))?)
                .ok_or(anyhow!("failed to deserialize as_to"))?,
            cc: object.as_cc.into(),
            end_time: object.as_end_time,
            published: object.as_published,
            one_of: object.as_one_of.and_then(from_serde),
            any_of: object.as_any_of.and_then(from_serde),
            content: object.as_content,
            content_map: object.as_content_map.and_then(from_serde),
            summary: object.as_summary,
            voters_count: object.ap_voters_count,
            url: object.as_url.and_then(from_serde),
            conversation: object.ap_conversation,
            tag: object.as_tag.and_then(from_serde),
            attachment: object.as_attachment.and_then(from_serde),
            sensitive: object.ap_sensitive,
            in_reply_to: object.as_in_reply_to.and_then(from_serde),
            ephemeral: Some(Ephemeral {
                created_at: Some(object.created_at),
                updated_at: Some(object.updated_at),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}
