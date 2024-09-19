use core::fmt;
use std::{collections::HashMap, fmt::Debug};

use crate::{
    activity_pub::{ApAttachment, ApContext, ApTag, Outbox},
    db::Db,
    fairings::events::EventChannels,
    models::{
        from_serde,
        from_time,
        pg::coalesced_activity::CoalescedActivity,
        profiles::Profile,
        remote_questions::RemoteQuestion, //timeline::ContextualizedTimelineItem,
    },
    MaybeMultiple,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};

use super::{
    actor::{ApActor, ApAddress},
    collection::ApCollectionType,
    note::{ApNoteType, Metadata},
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
    pub ephemeral_announces: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_actors: Option<Vec<ApActor>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_liked: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_announced: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_targeted: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_metadata: Option<Vec<Metadata>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_likes: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_created_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_updated_at: Option<DateTime<Utc>>,
}

impl ApQuestion {
    pub fn dedup(mut self) -> Self {
        if let Some(mut announces) = self.ephemeral_announces {
            announces.sort();
            announces.dedup();
            self.ephemeral_announces = Some(announces);
        }

        if let Some(mut likes) = self.ephemeral_likes {
            likes.sort();
            likes.dedup();
            self.ephemeral_likes = Some(likes);
        }

        self
    }
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

impl TryFrom<CoalescedActivity> for ApQuestion {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        let kind = coalesced
            .object_type
            .ok_or_else(|| anyhow::anyhow!("object_type is None"))?
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to convert object_type: {}", e))?;

        let id = coalesced
            .object_id
            .ok_or_else(|| anyhow::anyhow!("object_id is None"))?;
        let url = coalesced.object_url;
        let to = coalesced
            .object_to
            .and_then(from_serde)
            .ok_or_else(|| anyhow::anyhow!("object_to is None"))?;
        let cc = coalesced.object_cc.and_then(from_serde);
        let tag = coalesced.object_tag.and_then(from_serde);
        let attributed_to = ApAddress::from(
            coalesced
                .object_attributed_to
                .ok_or_else(|| anyhow::anyhow!("object_attributed_to is None"))?,
        );
        let in_reply_to = coalesced.object_in_reply_to;
        let content = coalesced.object_content;
        let conversation = coalesced.object_conversation;
        let attachment = coalesced.object_attachment.and_then(from_serde);
        let summary = coalesced.object_summary;
        let sensitive = coalesced.object_sensitive;
        let published = coalesced
            .object_published
            .and_then(|x| Some(DateTime::parse_from_rfc3339(&x).ok()?.with_timezone(&Utc)));
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

impl TryFrom<RemoteQuestion> for ApQuestion {
    type Error = anyhow::Error;

    fn try_from(question: RemoteQuestion) -> Result<Self, Self::Error> {
        Ok(ApQuestion {
            id: question.ap_id,
            attributed_to: question.attributed_to.into(),
            to: from_serde(question.ap_to.ok_or(anyhow!("ap_to is None"))?)
                .ok_or(anyhow!("failed to deserialize ap_to"))?,
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
