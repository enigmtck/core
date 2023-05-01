use core::fmt;
use std::fmt::Debug;

use crate::activity_pub::{ApAddress, ApContext};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApRemoveType {
    #[default]
    Remove,
}

impl fmt::Display for ApRemoveType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApRemove {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApRemoveType,
    pub actor: ApAddress,
    pub target: Option<String>,
    pub object: String,
}
