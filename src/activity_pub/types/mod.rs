use crate::activity_pub::ApActorTerse;
use chrono::{DateTime, Utc};
use note::Metadata;
use serde::{Deserialize, Serialize};

pub mod accept;
pub mod activity;
pub mod actor;
pub mod add;
pub mod announce;
pub mod attachment;
pub mod block;
pub mod collection;
pub mod create;
pub mod delete;
pub mod follow;
//pub mod invite;
//pub mod join;
pub mod like;
pub mod note;
pub mod object;
pub mod question;
pub mod remove;
pub mod session;
pub mod signature;
pub mod undo;
pub mod update;

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct Ephemeral {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followers: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leaders: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leader_as_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_activity_as_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actors: Option<Vec<ApActorTerse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announces: Option<Vec<ApActorTerse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liked: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announced: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targeted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Vec<Metadata>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub likes: Option<Vec<ApActorTerse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributed_to: Option<Vec<ApActorTerse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

impl From<Option<Vec<ApActorTerse>>> for Ephemeral {
    fn from(actors: Option<Vec<ApActorTerse>>) -> Self {
        Self {
            actors,
            ..Default::default()
        }
    }
}
