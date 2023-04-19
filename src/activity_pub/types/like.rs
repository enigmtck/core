use core::fmt;
use std::fmt::Debug;

use crate::{
    // activity_pub::{ApActivity, ApActivityType, ApContext},
    activity_pub::{ApAddress, ApContext},
    models::likes::Like,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApLikeType {
    #[default]
    Like,
}

impl fmt::Display for ApLikeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApLike {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApLikeType,
    pub actor: ApAddress,
    #[serde(skip_serializing)]
    pub to: Option<ApAddress>,
    pub id: Option<String>,
    pub object: String,
}

impl From<Like> for ApLike {
    fn from(like: Like) -> Self {
        ApLike {
            context: Some(ApContext::default()),
            kind: ApLikeType::Like,
            actor: ApAddress::Address(like.actor),
            to: Some(ApAddress::Address(like.ap_to)),
            id: Some(format!("{}/likes/{}", *crate::SERVER_URL, like.uuid)),
            object: like.object_ap_id,
        }
    }
}
