use crate::activity_pub::{ApActor, ApBaseObject, ApContext, ApFlexible, ApObjectType, ApTag};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApNote {
    #[serde(flatten)]
    pub base: ApBaseObject,
    #[serde(rename = "type")]
    pub kind: ApObjectType,
    pub to: Vec<String>,
    pub content: String,
}

impl From<ApActor> for ApNote {
    fn from(actor: ApActor) -> Self {
        ApNote {
            base: ApBaseObject {
                tag: Option::from(vec![]),
                attributed_to: Some(ApFlexible::Single(serde_json::Value::from(
                    actor.base.id.unwrap(),
                ))),
                id: Option::None,
                ..Default::default()
            },
            kind: ApObjectType::Note,
            to: vec![],
            content: String::new(),
        }
    }
}

impl ApNote {
    pub fn to(mut self, to: String) -> Self {
        self.to.push(to);
        self
    }

    pub fn content(mut self, content: String) -> Self {
        self.content = content;
        self
    }

    pub fn tag(mut self, tag: ApTag) -> Self {
        self.base.tag.as_mut().expect("unwrap failed").push(tag);
        self
    }
}
