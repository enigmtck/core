use core::fmt;
use std::fmt::Debug;

use super::Ephemeral;
use crate::activity_pub::{
    ActivityPub, ApActivity, ApActor, ApActorTerse, ApContext, ApInstrument, ApObject, Outbox,
};
use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::helper::{get_followers_ap_id_from_username, get_following_ap_id_from_username};
use crate::models::cache::Cache;
use crate::models::{actors::Actor, followers::Follower, leaders::Leader};
use crate::routes::ActivityJson;
use crate::MaybeReference;
use anyhow::{anyhow, Result};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub trait Collectible {
    fn items(&self) -> Option<Vec<ActivityPub>>;
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
    #[serde(alias = "collection_page")]
    CollectionPage,
    #[serde(alias = "ordered_collection_page")]
    OrderedCollectionPage,
}

impl fmt::Display for ApCollectionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_items: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Vec<ActivityPub>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ordered_items: Option<Vec<ActivityPub>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<MaybeReference<Box<ApCollection>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<MaybeReference<Box<ApCollection>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<MaybeReference<Box<ApCollection>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<MaybeReference<Box<ApCollection>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<MaybeReference<Box<ApCollection>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<Ephemeral>,
}

impl Collectible for ApCollection {
    fn items(&self) -> Option<Vec<ActivityPub>> {
        self.items.clone().or(self.ordered_items.clone())
    }
}

impl Outbox for ApCollection {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Actor,
        _raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
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
            ephemeral: None,
        }
    }
}

impl ApCollection {
    pub fn total_items(&mut self, total: Option<i64>) -> &mut Self {
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

type ApCollectionParams = (i64, String);
impl From<ApCollectionParams> for ApCollection {
    fn from((count, base_url): ApCollectionParams) -> Self {
        ApCollection {
            kind: ApCollectionType::OrderedCollection,
            total_items: Some(count),
            first: Some(MaybeReference::from(format!("{base_url}?page=true"))),
            last: Some(MaybeReference::from(format!("{base_url}?page=true&min=0"))),
            ..Default::default()
        }
    }
}

type ApCollectionPageParams = (Vec<ActivityPub>, Option<String>);
impl From<ApCollectionPageParams> for ApCollection {
    fn from((objects, base_url): ApCollectionPageParams) -> Self {
        ApCollection {
            kind: ApCollectionType::OrderedCollectionPage,
            total_items: None,
            ordered_items: Some(objects.clone()),
            prev: base_url.clone().and_then(|y| {
                objects.first().map(|x| {
                    MaybeReference::from(format!("{y}&min={}", x.timestamp().timestamp_micros()))
                })
            }),
            next: base_url.clone().and_then(|y| {
                objects.last().map(|x| {
                    MaybeReference::from(format!("{y}&max={}", x.timestamp().timestamp_micros()))
                })
            }),
            part_of: base_url,
            ..Default::default()
        }
    }
}

impl From<Vec<ApInstrument>> for ApCollection {
    fn from(instruments: Vec<ApInstrument>) -> Self {
        Self {
            kind: ApCollectionType::Collection,
            total_items: Some(instruments.len() as i64),
            items: Some(instruments.into_iter().map(ApObject::from).collect()),
            ..Default::default()
        }
    }
}

#[derive(Clone)]
pub struct ActorsPage {
    pub page: u32,
    pub profile: Actor,
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
                    request.profile.ek_username.unwrap()
                )),
                total_items: Some(request.actors.len() as i64),
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
    pub page: Option<u32>,
    pub profile: Actor,
    pub total_items: i64,
    pub followers: Vec<Follower>,
    pub actors: Option<Vec<ApActorTerse>>,
}

impl TryFrom<FollowersPage> for ApCollection {
    type Error = anyhow::Error;

    fn try_from(request: FollowersPage) -> Result<Self> {
        let id = get_followers_ap_id_from_username(
            request
                .profile
                .ek_username
                .ok_or(anyhow!("USERNAME CAN NOT BE NONE"))?,
        );

        fn get_last(total_items: i64, page_size: u8) -> i64 {
            let remainder = if total_items % 20 > 0 { 1 } else { 0 };

            (total_items / (page_size as i64)) + remainder
        }

        match request.page {
            Some(page) => Ok(ApCollection {
                kind: ApCollectionType::OrderedCollectionPage,
                id: Some(format!("{id}?page={page}")),
                total_items: Some(request.total_items),
                first: Some(format!("{id}?page=1").into()),
                last: Some(format!("{id}?page={}", get_last(request.total_items, 20)).into()),
                next: ((page as i64) < get_last(request.total_items, 20))
                    .then_some(format!("{id}?page={}", page + 1).into()),
                prev: (page > 1).then_some(format!("{id}?page={}", page - 1).into()),
                part_of: Some(id.clone()),
                ordered_items: Some(
                    request
                        .followers
                        .into_iter()
                        .map(|x| ActivityPub::Object(ApObject::Plain(x.actor)))
                        .collect::<Vec<ActivityPub>>(),
                ),
                ephemeral: Some(request.actors.into()),
                ..Default::default()
            }),
            None => Ok(ApCollection {
                kind: ApCollectionType::OrderedCollection,
                id: Some(id.clone()),
                total_items: Some(request.total_items),
                first: Some(format!("{id}?page=1").into()),
                last: Some(format!("{id}?page={}", get_last(request.total_items, 20)).into()),
                next: Some(format!("{id}?page=1").into()),
                part_of: Some(id.clone()),
                ordered_items: None,
                ephemeral: None,
                ..Default::default()
            }),
        }
    }
}

#[derive(Clone)]
pub struct LeadersPage {
    pub page: Option<u32>,
    pub profile: Actor,
    pub total_items: i64,
    pub leaders: Vec<Leader>,
    pub actors: Option<Vec<ApActorTerse>>,
}

impl TryFrom<LeadersPage> for ApCollection {
    type Error = anyhow::Error;

    fn try_from(request: LeadersPage) -> Result<Self> {
        let id = get_following_ap_id_from_username(
            request
                .profile
                .ek_username
                .ok_or(anyhow!("USERNAME CAN NOT BE NONE"))?,
        );

        fn get_last(total_items: i64, page_size: u8) -> i64 {
            let remainder = if total_items % 20 > 0 { 1 } else { 0 };

            (total_items / (page_size as i64)) + remainder
        }

        match request.page {
            Some(page) => Ok(ApCollection {
                kind: ApCollectionType::OrderedCollectionPage,
                id: Some(format!("{id}?page={page}")),
                total_items: Some(request.total_items),
                first: Some(format!("{id}?page=1").into()),
                last: Some(format!("{id}?page={}", get_last(request.total_items, 20)).into()),
                next: ((page as i64) < get_last(request.total_items, 20))
                    .then_some(format!("{id}?page={}", page + 1).into()),
                prev: (page > 1).then_some(format!("{id}?page={}", page - 1).into()),
                part_of: Some(id.clone()),
                ordered_items: Some(
                    request
                        .leaders
                        .into_iter()
                        .map(|x| ActivityPub::Object(ApObject::Plain(x.leader_ap_id)))
                        .collect::<Vec<ActivityPub>>(),
                ),
                ephemeral: Some(request.actors.into()),
                ..Default::default()
            }),
            None => Ok(ApCollection {
                kind: ApCollectionType::OrderedCollection,
                id: Some(id.clone()),
                total_items: Some(request.total_items),
                first: Some(format!("{id}?page=1").into()),
                last: Some(format!("{id}?page={}", get_last(request.total_items, 20)).into()),
                next: Some(format!("{id}?page=1").into()),
                part_of: Some(id.clone()),
                ordered_items: None,
                ephemeral: None,
                ..Default::default()
            }),
        }
    }
}
