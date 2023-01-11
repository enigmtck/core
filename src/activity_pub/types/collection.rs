use crate::activity_pub::{ApBaseObjectType, ApContext, ApObject};
use crate::models::{followers::Follower, leaders::Leader, profiles::Profile};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApCollection {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "@context")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApBaseObjectType,
    pub id: Option<String>,
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

impl Default for ApCollection {
    fn default() -> ApCollection {
        ApCollection {
            context: Option::from(ApContext::Plain(
                "https://www.w3.org/ns/activitystreams".to_string(),
            )),
            kind: ApBaseObjectType::Collection,
            id: Option::from(format!(
                "https://{}/collections/{}",
                *crate::SERVER_NAME,
                Uuid::new_v4()
            )),
            total_items: 0,
            items: Option::None,
            first: Option::None,
            last: Option::None,
            next: Option::None,
            current: Option::None,
            part_of: Option::None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApOrderedCollection {
    #[serde(flatten)]
    pub base: ApCollection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordered_items: Option<Vec<ApObject>>,
}

impl From<Vec<ApObject>> for ApCollection {
    fn from(objects: Vec<ApObject>) -> Self {
        ApCollection {
            total_items: objects.len() as u32,
            items: Option::from(objects),
            ..Default::default()
        }
    }
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
                    kind: ApBaseObjectType::OrderedCollection,
                    id: Option::from(format!(
                        "{}/users/{}/followers",
                        *crate::SERVER_URL,
                        request.profile.username
                    )),
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
                    part_of: Option::None,
                    ..Default::default()
                },
                ordered_items: Option::None,
            }
        }
    }
}

#[derive(Clone)]
pub struct LeadersPage {
    pub page: u32,
    pub profile: Profile,
    pub leaders: Vec<Leader>,
}

impl From<LeadersPage> for ApOrderedCollection {
    fn from(request: LeadersPage) -> Self {
        if request.page == 0 {
            ApOrderedCollection {
                base: ApCollection {
                    kind: ApBaseObjectType::OrderedCollection,
                    id: Option::from(format!(
                        "{}/users/{}/following",
                        *crate::SERVER_URL,
                        request.profile.username
                    )),
                    total_items: request.leaders.len() as u32,
                    first: Option::None,
                    part_of: Option::None,
                    ..Default::default()
                },
                ordered_items: Option::from(
                    request
                        .leaders
                        .into_iter()
                        .map(|x| ApObject::Plain(x.leader_ap_id))
                        .collect::<Vec<ApObject>>(),
                ),
            }
        } else {
            ApOrderedCollection {
                base: ApCollection {
                    part_of: Option::None,
                    ..Default::default()
                },
                ordered_items: Option::None,
            }
        }
    }
}
