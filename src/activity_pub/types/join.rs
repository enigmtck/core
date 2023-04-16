use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApContext, ApInstruments, ApObject, ApSession},
    MaybeMultiple, MaybeReference,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApJoinType {
    #[default]
    Join,
}

impl fmt::Display for ApJoinType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApJoin {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApJoinType,
    pub actor: String,
    pub id: Option<String>,
    pub to: MaybeMultiple<String>,
    pub object: MaybeReference<ApObject>,
}

impl TryFrom<ApSession> for ApJoin {
    type Error = &'static str;
    fn try_from(session: ApSession) -> Result<Self, Self::Error> {
        if let ApInstruments::Multiple(_) = session.instrument {
            Ok(ApJoin {
                context: Some(ApContext::default()),
                kind: ApJoinType::default(),
                actor: session.attributed_to.clone(),
                id: session.id.clone().map(|id| format!("{id}#join")),
                to: session.to.clone().into(),
                object: ApObject::Session(session).into(),
            })
        } else {
            Err("SINGLE INSTRUMENT DETECTED - THIS LOOKS LIKE AN INVITE")
        }
    }
}
