use core::fmt;
use std::fmt::Debug;

use crate::activity_pub::ApContext;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApFollowType {
    #[default]
    Follow,
}

impl fmt::Display for ApFollowType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApFollow {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApFollowType,
    pub actor: String,
    pub id: Option<String>,
    pub object: String,
}
