use core::fmt;
use std::cmp::Reverse;
use std::fmt::Debug;

use crate::activity_pub::{ActivityPub, ApActivity, ApActor, ApContext, ApObject, Outbox};
use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::models::cache::Cache;
use crate::models::vault::VaultItem;
use crate::models::{followers::Follower, leaders::Leader, profiles::Profile};
use crate::MaybeReference;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub trait Collectible {
    fn items(&self) -> Option<Vec<ActivityPub>>;
}

impl Cache for ApCollectionPage {
    async fn cache(&self, conn: &Db) -> &Self {
        let items = self.items().unwrap_or_default();
        for item in items {
            if let ActivityPub::Activity(ApActivity::Create(create)) = item {
                if let MaybeReference::Actual(ApObject::Note(note)) = create.object {
                    note.cache(conn).await;
                }
            }
        }

        self
    }
}

impl Cache for ApCollection {
    async fn cache(&self, conn: &Db) -> &Self {
        let items = self.items().unwrap_or_default();
        for item in items {
            if let ActivityPub::Activity(ApActivity::Create(create)) = item {
                if let MaybeReference::Actual(ApObject::Note(note)) = create.object {
                    note.cache(conn).await;
                }
            }
        }

        self
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApCollectionType {
    #[default]
    #[serde(alias = "collection")]
    Collection,
    #[serde(alias = "ordered_collection")]
    OrderedCollection,
}

impl fmt::Display for ApCollectionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApCollectionPageType {
    #[default]
    #[serde(alias = "collection_page")]
    CollectionPage,
    #[serde(alias = "ordered_collection_page")]
    OrderedCollectionPage,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApCollectionPage {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "@context")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApCollectionPageType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_items: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Vec<ActivityPub>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ordered_items: Option<Vec<ActivityPub>>,
}

impl Collectible for ApCollectionPage {
    fn items(&self) -> Option<Vec<ActivityPub>> {
        self.items.clone().map_or(self.ordered_items.clone(), Some)
    }
}

impl Outbox for ApCollectionPage {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Profile,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl Default for ApCollectionPage {
    fn default() -> ApCollectionPage {
        ApCollectionPage {
            context: Some(ApContext::default()),
            kind: ApCollectionPageType::default(),
            id: Some(format!(
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
            part_of: None,
        }
    }
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
    items: Option<Vec<ActivityPub>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ordered_items: Option<Vec<ActivityPub>>,
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

impl Collectible for ApCollection {
    fn items(&self) -> Option<Vec<ActivityPub>> {
        self.items.clone().and_then(|_| self.ordered_items.clone())
    }
}

impl Outbox for ApCollection {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Profile,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl Default for ApCollection {
    fn default() -> ApCollection {
        ApCollection {
            context: Some(ApContext::default()),
            kind: ApCollectionType::Collection,
            id: Some(format!(
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

impl ApCollection {
    pub fn total_items(&mut self, total: Option<u32>) -> &mut Self {
        self.total_items = total;
        self
    }

    pub fn ordered(&mut self) -> &mut Self {
        self.kind = ApCollectionType::OrderedCollection;
        self
    }

    pub fn first(&mut self, first: String) -> &mut Self {
        self.first = Some(MaybeReference::Reference(first));
        self
    }

    pub fn last(&mut self, last: String) -> &mut Self {
        self.last = Some(MaybeReference::Reference(last));
        self
    }

    pub fn id(&mut self, id: String) -> &mut Self {
        self.id = Some(id);
        self
    }
}

impl From<Vec<ActivityPub>> for ApCollection {
    fn from(objects: Vec<ActivityPub>) -> Self {
        ApCollection {
            total_items: Some(objects.len() as u32),
            ordered_items: Some(objects),
            ..Default::default()
        }
    }
}

type ApCollectionParams = (Vec<ActivityPub>, Option<String>);

impl From<ApCollectionParams> for ApCollection {
    fn from((mut objects, base_url): ApCollectionParams) -> Self {
        objects.sort_by_key(|x| Reverse(x.timestamp()));

        ApCollection {
            kind: ApCollectionType::OrderedCollection,
            total_items: Some(objects.len() as u32),
            ordered_items: Some(objects.clone()),
            prev: base_url.clone().and_then(|y| {
                objects
                    .first()
                    .map(|x| format!("{y}&min={}", x.timestamp().timestamp_micros()).into())
            }),
            next: base_url.and_then(|y| {
                objects
                    .last()
                    .map(|x| format!("{y}&max={}", x.timestamp().timestamp_micros()).into())
            }),
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
                id: Some(format!(
                    "{}/users/{}/actors",
                    *crate::SERVER_URL,
                    request.profile.username
                )),
                total_items: Some(request.actors.len() as u32),
                first: None,
                part_of: None,
                ordered_items: Some(
                    request
                        .actors
                        .into_iter()
                        .map(|x| ActivityPub::Object(ApObject::Actor(x)))
                        .collect::<Vec<ActivityPub>>(),
                ),
                ..Default::default()
            }
        } else {
            ApCollection {
                part_of: None,
                ordered_items: None,
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
                id: Some(format!(
                    "{}/users/{}/followers",
                    *crate::SERVER_URL,
                    request.profile.username
                )),
                total_items: Some(request.followers.len() as u32),
                first: None,
                part_of: None,
                ordered_items: Some(
                    request
                        .followers
                        .into_iter()
                        .map(|x| ActivityPub::Object(ApObject::Plain(x.actor)))
                        .collect::<Vec<ActivityPub>>(),
                ),
                ..Default::default()
            }
        } else {
            ApCollection {
                part_of: None,
                ordered_items: None,
                ..Default::default()
            }
        }
    }
}

#[derive(Clone)]
pub struct ActivitiesPage {
    pub profile: Profile,
    pub activities: Vec<ApActivity>,
    pub first: Option<String>,
    pub last: Option<String>,
    pub next: Option<String>,
    pub prev: Option<String>,
    pub part_of: Option<String>,
}

impl From<ActivitiesPage> for ApCollectionPage {
    fn from(request: ActivitiesPage) -> Self {
        ApCollectionPage {
            kind: ApCollectionPageType::OrderedCollectionPage,
            id: Some(format!(
                "{}/users/{}/activities",
                *crate::SERVER_URL,
                request.profile.username
            )),
            first: request.first,
            last: request.last,
            next: request.next,
            prev: request.prev,
            part_of: request.part_of,
            ordered_items: Some(
                request
                    .activities
                    .into_iter()
                    .map(ActivityPub::Activity)
                    .collect::<Vec<ActivityPub>>(),
            ),
            ..Default::default()
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
                id: Some(format!(
                    "{}/users/{}/following",
                    *crate::SERVER_URL,
                    request.profile.username
                )),
                total_items: Some(request.leaders.len() as u32),
                first: None,
                part_of: None,
                ordered_items: Some(
                    request
                        .leaders
                        .into_iter()
                        .map(|x| ActivityPub::Object(ApObject::Plain(x.leader_ap_id)))
                        .collect::<Vec<ActivityPub>>(),
                ),
                ..Default::default()
            }
        } else {
            ApCollection {
                part_of: None,
                ordered_items: None,
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
            id: Some(format!(
                "{}/ephemeral-collection/{}",
                *crate::SERVER_URL,
                uuid::Uuid::new_v4()
            )),
            total_items: Some(items.len() as u32),
            first: None,
            part_of: None,
            ordered_items: Some(
                items
                    .into_iter()
                    .map(|x| ActivityPub::Object(ApObject::Note((x, profile.clone()).into())))
                    .collect::<Vec<ActivityPub>>(),
            ),
            ..Default::default()
        }
    }
}
