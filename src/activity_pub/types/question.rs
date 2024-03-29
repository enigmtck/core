use core::fmt;
use std::{collections::HashMap, fmt::Debug};

use crate::{
    activity_pub::{ApAttachment, ApContext, ApTag, Outbox},
    db::Db,
    fairings::events::EventChannels,
    models::{
        from_serde, from_time, profiles::Profile, remote_questions::RemoteQuestion,
        timeline::ContextualizedTimelineItem,
    },
    MaybeMultiple,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};

use super::{actor::ApAddress, collection::ApCollectionType, note::ApNoteType};

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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<MaybeMultiple<ApAddress>>,

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
    pub ephemeral_created_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_updated_at: Option<DateTime<Utc>>,
}

impl Outbox for ApQuestion {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Profile,
    ) -> Result<String, Status> {
        Err(Status::NotImplemented)
    }
}

impl TryFrom<RemoteQuestion> for ApQuestion {
    type Error = anyhow::Error;

    fn try_from(question: RemoteQuestion) -> Result<Self, Self::Error> {
        Ok(ApQuestion {
            id: question.ap_id,
            attributed_to: question.attributed_to.into(),
            to: from_serde(question.ap_to.ok_or(Self::Error::msg("ap_to is None"))?)
                .ok_or(Self::Error::msg("failed to deserialize ap_to"))?,
            cc: question.cc.and_then(from_serde),
            end_time: question.end_time.and_then(from_time),
            published: question.published.and_then(from_time),
            one_of: question.one_of.and_then(from_serde),
            any_of: question.any_of.and_then(from_serde),
            content: question.content,
            content_map: question.content_map.and_then(from_serde),
            summary: question.summary,
            voters_count: question.voters_count,
            url: question.url,
            conversation: question.conversation,
            tag: question.tag.and_then(from_serde),
            attachment: question.attachment.and_then(from_serde),
            sensitive: question.ap_sensitive,
            in_reply_to: question.in_reply_to,
            ephemeral_created_at: from_time(question.created_at),
            ephemeral_updated_at: from_time(question.updated_at),
            ..Default::default()
        })
    }
}

impl TryFrom<ContextualizedTimelineItem> for ApQuestion {
    type Error = anyhow::Error;
    fn try_from(
        ContextualizedTimelineItem {
            item,
            activity,
            cc,
            related,
            requester,
        }: ContextualizedTimelineItem,
    ) -> Result<Self, Self::Error> {
        if item.kind.to_string().to_lowercase().as_str() == "question" {
            Ok(ApQuestion {
                context: Some(ApContext::default()),
                to: MaybeMultiple::Multiple(vec![]),
                cc: None,
                kind: ApQuestionType::Question,
                tag: item.tag.and_then(from_serde),
                attributed_to: ApAddress::Address(item.attributed_to),
                id: item.ap_id,
                url: item.url,
                published: item.published.and_then(|x| x.parse().ok()),
                in_reply_to: item.in_reply_to,
                content: item.content,
                summary: item.summary,
                end_time: item.end_time.and_then(from_time),
                one_of: item.one_of.and_then(from_serde),
                any_of: item.any_of.and_then(from_serde),
                voters_count: item.voters_count,
                sensitive: item.ap_sensitive,
                conversation: item.conversation,
                content_map: item.content_map.and_then(from_serde),
                attachment: item.attachment.and_then(from_serde),
                ephemeral_created_at: from_time(item.created_at),
                ephemeral_updated_at: from_time(item.updated_at),
            })
        } else {
            Err(anyhow::Error::msg("wrong timeline_item type"))
        }
    }
}
