use crate::activity_pub::Note;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Object {
    Note(Note),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Create {
    #[serde(rename = "@context")]
    pub context: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub id: String,
    pub to: Vec<String>,
    pub actor: String,
    pub object: Object,
}

impl From<Note> for Create {
    fn from(note: Note) -> Self {
        let mut note = note;
        note.context = Option::None;
        note.id = Option::from(format!("{}/posts/{}", note.attributed_to, Uuid::new_v4()));

        Create {
            context: "https://www.w3.org/ns/activitystreams".to_string(),
            kind: "Create".to_string(),
            id: note.clone().id.unwrap(),
            to: note.clone().to,
            actor: note.clone().attributed_to,
            object: Object::Note(note),
        }
    }
}
