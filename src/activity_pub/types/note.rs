use crate::activity_pub::Actor;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub to: Vec<String>,
    pub attributed_to: String,
    pub content: String,
}

impl From<Actor> for Note {
    fn from(actor: Actor) -> Self {
        Note {
            context: Option::from("https://www.w3.org/ns/activitystreams".to_string()),
            kind: "Note".to_string(),
            id: Option::None,
            attributed_to: actor.id,
            to: vec![],
            content: String::new(),
        }
    }
}

impl Note {
    pub fn to(mut self, to: String) -> Self {
        self.to.push(to);
        self
    }

    pub fn content(mut self, content: String) -> Self {
        self.content = content;
        self
    }
}
