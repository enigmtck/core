use crate::activity_pub::{ApBaseObject, ApBaseObjectType, ApContext, ApObject};
use crate::models::{followers::Follower, profiles::Profile};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApCollection {
    #[serde(flatten)]
    pub base: ApBaseObject,
    pub total_items: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ApObject>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    part_of: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApOrderedCollection {
    #[serde(flatten)]
    pub base: ApCollection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordered_items: Option<Vec<ApObject>>,
}

#[derive(Clone)]
pub struct FollowersPage {
    pub page: u32,
    pub profile: Profile,
    pub followers: Vec<Follower>,
}

impl From<FollowersPage> for ApOrderedCollection {
    fn from(request: FollowersPage) -> Self {
        if request.page == 0 {
            ApOrderedCollection {
                base: ApCollection {
                    base: ApBaseObject {
                        context: Option::from(ApContext::Plain(
                            "https://www.w3.org/ns/activitystreams".to_string(),
                        )),
                        kind: Option::from(ApBaseObjectType::OrderedCollection),
                        id: Option::from(format!(
                            "https://enigmatick.jdt.dev/users/{}/followers",
                            request.profile.username
                        )),
                        ..Default::default()
                    },
                    total_items: request.followers.len() as u32,
                    first: Option::None,
                    part_of: Option::None,
                    ..Default::default()
                },
                ordered_items: Option::from(
                    request
                        .followers
                        .into_iter()
                        .map(|x| ApObject::Plain(x.actor))
                        .collect::<Vec<ApObject>>(),
                ),
            }
        } else {
            ApOrderedCollection {
                base: ApCollection {
                    base: ApBaseObject {
                        ..Default::default()
                    },
                    part_of: Option::None,
                    ..Default::default()
                },
                ordered_items: Option::None,
            }
        }
    }
}
