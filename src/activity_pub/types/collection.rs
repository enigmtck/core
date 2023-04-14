use core::fmt;
use std::fmt::Debug;

use crate::activity_pub::{ApActor, ApContext, ApObject};
use crate::models::vault::VaultItem;
use crate::models::{followers::Follower, leaders::Leader, profiles::Profile};
use crate::MaybeReference;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApCollectionType {
    Collection,
    OrderedCollection,
    #[default]
    Unknown,
}

impl fmt::Display for ApCollectionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApCollectionPageType {
    #[default]
    CollectionPage,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApCollectionPage {
    #[serde(rename = "type")]
    pub kind: ApCollectionPageType,
    pub next: Option<String>,
    pub part_of: Option<String>,
    pub items: Vec<ApObject>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApCollection {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "@context")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApCollectionType,
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_items: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ApObject>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordered_items: Option<Vec<ApObject>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<MaybeReference<ApCollectionPage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<MaybeReference<ApCollectionPage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<MaybeReference<ApCollectionPage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<MaybeReference<ApCollectionPage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<MaybeReference<ApCollectionPage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_of: Option<String>,
}

impl Default for ApCollection {
    fn default() -> ApCollection {
        ApCollection {
            context: Option::from(ApContext::default()),
            kind: ApCollectionType::Collection,
            id: Option::from(format!(
                "https://{}/collections/{}",
                *crate::SERVER_NAME,
                Uuid::new_v4()
            )),
            total_items: None,
            items: None,
            ordered_items: None,
            first: None,
            last: None,
            next: None,
            prev: None,
            current: None,
            part_of: None,
        }
    }
}

impl From<Vec<ApObject>> for ApCollection {
    fn from(objects: Vec<ApObject>) -> Self {
        ApCollection {
            total_items: Some(objects.len() as u32),
            items: Option::from(objects),
            ..Default::default()
        }
    }
}

#[derive(Clone)]
pub struct ActorsPage {
    pub page: u32,
    pub profile: Profile,
    pub actors: Vec<ApActor>,
}

impl From<ActorsPage> for ApCollection {
    fn from(request: ActorsPage) -> Self {
        if request.page == 0 {
            ApCollection {
                kind: ApCollectionType::OrderedCollection,
                id: Option::from(format!(
                    "{}/users/{}/actors",
                    *crate::SERVER_URL,
                    request.profile.username
                )),
                total_items: Some(request.actors.len() as u32),
                first: Option::None,
                part_of: Option::None,
                ordered_items: Option::from(
                    request
                        .actors
                        .into_iter()
                        .map(ApObject::Actor)
                        .collect::<Vec<ApObject>>(),
                ),
                ..Default::default()
            }
        } else {
            ApCollection {
                part_of: Option::None,
                ordered_items: Option::None,
                ..Default::default()
            }
        }
    }
}

#[derive(Clone)]
pub struct FollowersPage {
    pub page: u32,
    pub profile: Profile,
    pub followers: Vec<Follower>,
}

impl From<FollowersPage> for ApCollection {
    fn from(request: FollowersPage) -> Self {
        if request.page == 0 {
            ApCollection {
                kind: ApCollectionType::OrderedCollection,
                id: Option::from(format!(
                    "{}/users/{}/followers",
                    *crate::SERVER_URL,
                    request.profile.username
                )),
                total_items: Some(request.followers.len() as u32),
                first: Option::None,
                part_of: Option::None,
                ordered_items: Option::from(
                    request
                        .followers
                        .into_iter()
                        .map(|x| ApObject::Plain(x.actor))
                        .collect::<Vec<ApObject>>(),
                ),
                ..Default::default()
            }
        } else {
            ApCollection {
                part_of: Option::None,
                ordered_items: Option::None,
                ..Default::default()
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

impl From<LeadersPage> for ApCollection {
    fn from(request: LeadersPage) -> Self {
        if request.page == 0 {
            ApCollection {
                kind: ApCollectionType::OrderedCollection,
                id: Option::from(format!(
                    "{}/users/{}/following",
                    *crate::SERVER_URL,
                    request.profile.username
                )),
                total_items: Some(request.leaders.len() as u32),
                first: Option::None,
                part_of: Option::None,
                ordered_items: Option::from(
                    request
                        .leaders
                        .into_iter()
                        .map(|x| ApObject::Plain(x.leader_ap_id))
                        .collect::<Vec<ApObject>>(),
                ),
                ..Default::default()
            }
        } else {
            ApCollection {
                part_of: Option::None,
                ordered_items: Option::None,
                ..Default::default()
            }
        }
    }
}

pub type IdentifiedVaultItems = (Vec<VaultItem>, Profile);

impl From<IdentifiedVaultItems> for ApCollection {
    fn from((items, profile): IdentifiedVaultItems) -> Self {
        ApCollection {
            kind: ApCollectionType::OrderedCollection,
            id: Option::from(format!(
                "{}/ephemeral-collection/{}",
                *crate::SERVER_URL,
                uuid::Uuid::new_v4()
            )),
            total_items: Some(items.len() as u32),
            first: Option::None,
            part_of: Option::None,
            ordered_items: Option::from(
                items
                    .into_iter()
                    .map(|x| ApObject::Note((x, profile.clone()).into()))
                    .collect::<Vec<ApObject>>(),
            ),
            ..Default::default()
        }
    }
}
