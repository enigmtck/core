use crate::activity_pub::{ApActor, ApContext, ApFlexible, ApObjectType, ApTag};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApNote {
    // #[serde(flatten)]
    // pub base: ApBaseObject,
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    pub tag: Option<Vec<ApTag>>,
    pub attributed_to: Option<ApFlexible>,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: ApObjectType,
    pub to: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<ApFlexible>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replies: Option<ApFlexible>,
    pub content: String,
}

impl Default for ApNote {
    fn default() -> ApNote {
        ApNote {
            context: Option::from(ApContext::Plain(
                "https://www.w3.org/ns/activitystreams".to_string(),
            )),
            tag: Option::None,
            attributed_to: Option::None,
            id: Option::None,
            kind: ApObjectType::Note,
            to: vec![],
            url: Option::None,
            published: Option::None,
            cc: Option::None,
            replies: Option::None,
            content: String::new(),
        }
    }
}

impl From<ApActor> for ApNote {
    fn from(actor: ApActor) -> Self {
        ApNote {
            tag: Option::from(vec![]),
            attributed_to: Some(ApFlexible::Single(serde_json::Value::from(
                actor.id.unwrap(),
            ))),
            id: Option::None,
            kind: ApObjectType::Note,
            to: vec![],
            content: String::new(),
            ..Default::default()
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
        self.tag.as_mut().expect("unwrap failed").push(tag);
        self
    }
}
