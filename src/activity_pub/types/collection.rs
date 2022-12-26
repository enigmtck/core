use crate::activity_pub::{ApBaseObject, ApObject};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApCollection {
    #[serde(flatten)]
    pub base: ApBaseObject,
    pub total_items: Option<u32>,
    pub items: Option<Vec<ApObject>>,
    pub first: Option<String>,
    pub last: Option<String>,
    pub current: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApOrderedCollection {
    #[serde(flatten)]
    pub base: ApCollection,
    pub ordered_items: Option<Vec<ApObject>>,
}
