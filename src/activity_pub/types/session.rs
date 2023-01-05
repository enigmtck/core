use crate::activity_pub::{ApBaseObject, ApInstrument, ApObjectType};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApSession {
    #[serde(flatten)]
    pub base: ApBaseObject,
    #[serde(rename = "type")]
    pub kind: ApObjectType,
    pub to: String,
    pub attributed_to: String,
    pub instrument: ApInstrument,
}
