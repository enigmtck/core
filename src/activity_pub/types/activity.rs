use crate::activity_pub::ApBaseObject;

use crate::activity_pub::{ApActivityType, ApContext, ApFlexible, ApNote, ApObject};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt::Debug;
use uuid::Uuid;

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApActivity {
    #[serde(flatten)]
    pub base: ApBaseObject,
    #[serde(rename = "type")]
    pub kind: ApActivityType,
    pub actor: String,
    pub object: ApObject,
}

impl From<ApNote> for ApActivity {
    fn from(note: ApNote) -> Self {
        let mut note = note;
        note.base.context = Option::None;
        let uuid = Uuid::new_v4().to_string();

        if let Some(ApFlexible::Single(attributed_to)) = note.clone().base.attributed_to {
            let attributed_to = attributed_to.as_str().unwrap();

            note.base.id = Option::from(format!("{}/posts/{}", attributed_to, uuid));

            ApActivity {
                base: ApBaseObject {
                    context: Option::from(ApContext::Plain(
                        "https://www.w3.org/ns/activitystreams".to_string(),
                    )),
                    to: note.clone().base.to,
                    kind: Option::None,
                    id: note.clone().base.id,
                    uuid: Option::from(uuid),
                    ..Default::default()
                },
                kind: ApActivityType::Create,
                actor: attributed_to.to_string(),
                object: ApObject::Note(note),
            }
        } else {
            ApActivity {
                ..Default::default()
            }
        }
    }
}
