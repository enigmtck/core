#![deny(unused_crate_dependencies)]
#![allow(async_fn_in_trait)]
extern crate diesel;
extern crate log;
// #[macro_use]
use crate::db::runner::DbRunner;
use crate::models::actors::{get_actor_by_as_id, Actor};
use crate::webfinger::retrieve_webfinger;
use atty as _;
use clap as _;
use comfy_table as _;
use crossterm as _;
use ctrlc as _;
use diesel_migrations as _;
use dotenvy::dotenv;
use env_logger as _;
use indicatif as _;
use jdt_activity_pub::{
    ActivityPub, ApActivity, ApActor, ApArticle, ApNote, ApObject, ApQuestion, ApTag, Ephemeral,
};
use jdt_activity_pub::{ApActorTerse, MaybeMultiple};
use jdt_activity_pub::{ApCollection, MaybeReference};
use lazy_static::lazy_static;
use log4rs as _;
use models::activities::{get_announced, get_announcers, get_liked, get_likers};
use models::actors::guaranteed_actor;
use models::follows::{
    get_follow, get_follower_count_by_actor_id, get_leader_count_by_follower_actor_id,
};
use models::objects::get_object_by_as_id;
use regex::Regex;
use reqwest::StatusCode;
use retriever::{collection_fetcher, signed_get};
use runner::note::{fetch_remote_object, handle_object};
use rust_embed as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;
use tower as _;
use tower_http as _;

#[cfg(feature = "vendored-openssl")]
use openssl as _;

#[cfg(feature = "bundled-postgres")]
use pq_sys as _;

pub mod admin;
pub mod blocklist;
pub mod db;
pub mod events;
pub mod helper;
pub mod models;
pub mod retriever;
pub mod runner;

#[cfg(all(feature = "pg", feature = "sqlite"))]
compile_error!("Features 'pg' and 'sqlite' cannot be enabled at the same time.");

#[cfg(feature = "pg")]
#[path = "schema-pg.rs"]
pub mod schema;

#[cfg(feature = "sqlite")]
#[path = "schema-sqlite.rs"]
pub mod schema;

pub mod server;
pub mod signing;
pub mod webfinger;

lazy_static! {
    pub static ref IMAGE_MEDIA_RE: Regex =
        Regex::new(r#"^image\/([a-z]+)$"#).expect("invalid image media regex");
    pub static ref ANCHOR_RE: Regex =
        Regex::new(r#"<a href="(.+?)".*?>"#).expect("invalid anchor regex");
    pub static ref WEBFINGER_RE: Regex =
        Regex::new(r#"@(.+?)@(.+)"#).expect("invalid webfinger regex");
    pub static ref WEBFINGER_ACCT_RE: Regex =
        Regex::new(r#"acct:(.+?)@(.+)"#).expect("invalid webfinger acct regex");
    pub static ref LOCAL_RE: Regex =
        Regex::new(&format!(r#"\w+?://{}/(.+)"#, *SERVER_NAME)).expect("invalid local regex");
    pub static ref DOMAIN_RE: Regex =
        Regex::new(r#"https://([\w\.-]+)/?(.*)"#).expect("invalid domain regex");
    pub static ref LOCAL_URL_RE: Regex = Regex::new(&format!(
        r#"^https://{}/(user|notes|session|collections|activities|objects|instruments)/(.+)$"#,
        *SERVER_NAME
    ))
    .expect("invalid local url regex");
    pub static ref LOCAL_USER_KEY_ID_RE: Regex =
        Regex::new(&format!(r#"(\w+://{}/user/(.+?))#(.+)"#, *SERVER_NAME))
            .expect("invalid local user key id regex");
    pub static ref ASSIGNMENT_RE: Regex =
        Regex::new(r#"(\w+)="(.+?)""#).expect("invalid assignment regex");
    pub static ref ACME_PROXY: bool = {
        dotenv().ok();
        env::var("ACME_PROXY").is_ok_and(|x| x.parse().expect("ACME_PROXY must be \"true\" or \"false\""))
    };
    pub static ref ACME_EMAILS: Option<Vec<String>> = {
        dotenv().ok();
        if let Ok(emails) = env::var("ACME_EMAIL") {
            serde_json::from_str(&emails).ok()
        } else {
            None
        }
    };
    pub static ref ACME_PORT: String = {
        dotenv().ok();
        env::var("ACME_PORT").unwrap_or("443".to_string())
    };
    pub static ref ROCKET_PORT: String = {
        dotenv().ok();
        env::var("ROCKET_PORT").unwrap_or("8000".to_string())
    };
    pub static ref ROCKET_ADDRESS: String = {
        dotenv().ok();
        env::var("ROCKET_ADDRESS").unwrap_or("0.0.0.0".to_string())
    };
    pub static ref SERVER_ADDRESS: String = {
        dotenv().ok();
        env::var("SERVER_ADDRESS").unwrap_or("0.0.0.0:8001".to_string())
    };
    pub static ref SERVER_NAME: String = {
        dotenv().ok();
        env::var("SERVER_NAME").expect("SERVER_NAME must be set")
    };
    pub static ref SYSTEM_USER: String = {
        dotenv().ok();
        env::var("SYSTEM_USER").expect("SYSTEM_USER must be set")
    };
    pub static ref MEDIA_DIR: String = {
        dotenv().ok();
        env::var("MEDIA_DIR").expect("MEDIA_DIR must be set")
    };
    pub static ref REGISTRATION_ENABLED: bool = {
        dotenv().ok();
        env::var("REGISTRATION_ENABLED")
            .expect("REGISTRATION_ENABLED must be set")
            .parse()
            .expect("REGISTRATION_ENABLED must be true or false")
    };
    pub static ref REGISTRATION_APPROVAL_REQUIRED: bool = {
        dotenv().ok();
        env::var("REGISTRATION_APPROVAL_REQUIRED")
            .expect("REGISTRATION_APPROVAL_REQUIRED must be set")
            .parse()
            .expect("REGISTRATION_APPROVAL_REQUIRED must be true or false")
    };
    pub static ref REGISTRATION_MESSAGE: String = {
        dotenv().ok();
        env::var("REGISTRATION_MESSAGE").expect("REGISTRATION_MESSAGE must be set")
    };
    pub static ref INSTANCE_CONTACT: String = {
        dotenv().ok();
        env::var("INSTANCE_CONTACT").expect("INSTANCE_CONTACT must be set")
    };
    pub static ref INSTANCE_TITLE: String = {
        dotenv().ok();
        env::var("INSTANCE_TITLE").expect("INSTANCE_TITLE must be set")
    };
    pub static ref INSTANCE_VERSION: String = {
        dotenv().ok();
        env::var("INSTANCE_VERSION").expect("INSTANCE_VERSION must be set")
    };
    pub static ref INSTANCE_SOURCE_URL: String = {
        dotenv().ok();
        env::var("INSTANCE_SOURCE_URL").expect("INSTANCE_SOURCE_URL must be set")
    };
    pub static ref INSTANCE_DESCRIPTION: String = {
        dotenv().ok();
        env::var("INSTANCE_DESCRIPTION").expect("INSTANCE_DESCRIPTION must be set")
    };

    // SIGNING_OVERRIDE turns off signature checking so that I can test the API using curl
    pub static ref SIGNING_OVERRIDE: bool = {
        dotenv().ok();
        env::var("SIGNING_OVERRIDE")
            .ok()
            .and_then(|x| x.parse().ok())
            .unwrap_or(false)
    };

    // CUSTOM_INDEX_PATH allows deployers to provide a custom landing page at /
    pub static ref CUSTOM_INDEX_PATH: Option<PathBuf> = {
        dotenv().ok();
        env::var("CUSTOM_INDEX_PATH").ok().map(PathBuf::from)
    };

    // CUSTOM_STATIC_DIR is automatically derived from CUSTOM_INDEX_PATH to serve custom assets
    pub static ref CUSTOM_STATIC_DIR: Option<PathBuf> = {
        CUSTOM_INDEX_PATH.as_ref().and_then(|p| p.parent().map(|p| p.to_path_buf()))
    };
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct OrdValue(Value);

impl PartialOrd for OrdValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrdValue {
    fn cmp(&self, other: &Self) -> Ordering {
        let a_str = serde_json::to_string(&self.0).unwrap();
        let b_str = serde_json::to_string(&other.0).unwrap();
        a_str.cmp(&b_str)
    }
}

pub trait GetWebfinger {
    async fn get_webfinger(&self) -> Option<String>;
}

impl GetWebfinger for ApActor {
    async fn get_webfinger(&self) -> Option<String> {
        let id = self.id.clone()?.to_string();
        let domain = DOMAIN_RE.captures(&id)?.get(1)?.as_str().to_string();
        let username = self.preferred_username.clone();

        let webfinger = retrieve_webfinger(domain, username).await.ok()?;

        webfinger.get_address()
    }
}

pub trait GetHashtags {
    fn get_hashtags(&self) -> Vec<String>;
}

impl GetHashtags for ApActor {
    fn get_hashtags(&self) -> Vec<String> {
        if let MaybeMultiple::Multiple(tags) = self.tag.clone() {
            tags.iter()
                .filter_map(|tag| {
                    if let ApTag::Hashtag(hashtag) = tag {
                        Some(hashtag.name.clone().to_lowercase())
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            vec![]
        }
    }
}

pub trait HasReplies {
    fn get_replies(&self) -> &MaybeReference<ApCollection>;
    fn get_replies_mut(&mut self) -> &mut MaybeReference<ApCollection>;
}

impl HasReplies for ApNote {
    fn get_replies(&self) -> &MaybeReference<ApCollection> {
        &self.replies
    }

    fn get_replies_mut(&mut self) -> &mut MaybeReference<ApCollection> {
        &mut self.replies
    }
}

impl HasReplies for ApArticle {
    fn get_replies(&self) -> &MaybeReference<ApCollection> {
        &self.replies
    }

    fn get_replies_mut(&mut self) -> &mut MaybeReference<ApCollection> {
        &mut self.replies
    }
}

impl HasReplies for ApQuestion {
    fn get_replies(&self) -> &MaybeReference<ApCollection> {
        &self.replies
    }

    fn get_replies_mut(&mut self) -> &mut MaybeReference<ApCollection> {
        &mut self.replies
    }
}

pub trait FetchReplies {
    async fn fetch_replies<C: DbRunner + Send + Sync>(
        &mut self,
        conn: &C,
        visited: &mut HashSet<String>,
    ) -> Self;
}

impl<T> FetchReplies for T
where
    T: HasReplies + Clone + Send + Sync,
{
    async fn fetch_replies<C: DbRunner + Send + Sync>(
        &mut self,
        conn: &C,
        visited: &mut HashSet<String>,
    ) -> Self {
        use futures_lite::StreamExt;

        let Some(replies_url) = self.get_replies().reference() else {
            return self.clone();
        };

        log::debug!("Fetching replies for {replies_url}");

        if visited.contains(&replies_url) {
            log::debug!("Already processed replies for {replies_url}");
            return self.clone();
        }
        visited.insert(replies_url.clone());

        let profile = guaranteed_actor(conn, None).await;

        let collection = match signed_get(profile.clone(), replies_url, false).await {
            Ok(resp) if matches!(resp.status(), StatusCode::ACCEPTED | StatusCode::OK) => {
                match resp.json::<Value>().await {
                    Ok(json) => serde_json::from_value::<ApCollection>(json).ok(),
                    Err(_) => None,
                }
            }
            _ => None,
        };

        let Some(collection) = collection else {
            return self.clone();
        };

        let mut stream = collection.stream_all(collection_fetcher());

        while let Some(Ok(ActivityPub::Object(ApObject::Note(note)))) = stream.next().await {
            let Some(note_id) = note.id.clone() else {
                continue;
            };

            // Skip if we already have this object
            if get_object_by_as_id(conn, note_id.clone()).await.is_ok() {
                continue;
            }

            log::debug!("Retrieving reply: {note:?}");

            if let Ok(object) = fetch_remote_object(conn, note_id, profile.clone()).await {
                let _ = Box::pin(handle_object(conn, object, visited)).await;
            }
        }

        self.clone()
    }
}

pub trait LoadEphemeral {
    async fn load_ephemeral<C: DbRunner + Send + Sync>(
        &mut self,
        conn: &C,
        requester: Option<Actor>,
    ) -> Self;
}

impl LoadEphemeral for ApNote {
    async fn load_ephemeral<C: DbRunner + Send + Sync>(
        &mut self,
        conn: &C,
        requester: Option<Actor>,
    ) -> Self {
        if let Ok(actor) = get_actor_by_as_id(conn, self.attributed_to.to_string()).await {
            let mut ephemeral = self.ephemeral.clone().unwrap_or_default();
            ephemeral.attributed_to = Some(vec![actor.into()]);

            ephemeral.announced =
                if let (Some(requester), Some(id)) = (requester.clone(), self.id.clone()) {
                    get_announced(conn, requester, id).await.unwrap_or(None)
                } else {
                    None
                };

            ephemeral.liked = if let (Some(requester), Some(id)) = (requester, self.id.clone()) {
                get_liked(conn, requester, id).await.unwrap_or(None)
            } else {
                None
            };

            self.ephemeral = Some(ephemeral);
        }

        self.clone()
    }
}

impl LoadEphemeral for ApActivity {
    async fn load_ephemeral<C: DbRunner + Send + Sync>(
        &mut self,
        conn: &C,
        requester: Option<Actor>,
    ) -> Self {
        match self.clone() {
            ApActivity::Create(mut create) => {
                if let MaybeReference::Actual(ApObject::Note(ref mut note)) = create.object {
                    note.load_ephemeral(conn, requester).await;
                }
                create.into()
            }
            ApActivity::Announce(mut announce) => {
                if let MaybeReference::Actual(ApObject::Note(ref mut note)) = announce.object {
                    note.load_ephemeral(conn, requester).await;
                }
                announce.into()
            }
            _ => self.clone(),
        }
    }
}

impl LoadEphemeral for ApActor {
    async fn load_ephemeral<C: DbRunner + Send + Sync>(
        &mut self,
        conn: &C,
        requester: Option<Actor>,
    ) -> Self {
        if let Some(ap_id) = self.id.clone() {
            if let Ok(profile) = get_actor_by_as_id(conn, ap_id.to_string()).await {
                let follow = if let (Some(requester), Some(id)) = (requester, self.id.clone()) {
                    get_follow(conn, requester.as_id, id.to_string()).await.ok()
                } else {
                    None
                };

                self.ephemeral = Some(Ephemeral {
                    follow_activity_as_id: follow.clone().and_then(|x| x.follow_activity_ap_id),
                    followers: if profile.ek_username.is_some() {
                        get_follower_count_by_actor_id(conn, profile.id).await.ok()
                    } else {
                        None
                    },
                    leaders: if profile.ek_username.is_some() {
                        get_leader_count_by_follower_actor_id(conn, profile.id)
                            .await
                            .ok()
                    } else {
                        None
                    },
                    summary_markdown: profile.ek_summary_markdown,
                    following: follow.map(|x| x.accepted),
                    ..Default::default()
                });
            }
        }

        self.clone()
    }
}

impl LoadEphemeral for ApObject {
    async fn load_ephemeral<C: DbRunner + Send + Sync>(
        &mut self,
        conn: &C,
        requester: Option<Actor>,
    ) -> Self {
        match self {
            ApObject::Note(ref mut note) => {
                let attributed_to = if let Ok(actor) =
                    retriever::get_actor(conn, note.attributed_to.clone().to_string(), None, true)
                        .await
                {
                    Some(vec![ApActorTerse::from(actor)])
                } else {
                    None
                };

                let announced =
                    if let (Some(requester), Some(id)) = (requester.clone(), note.id.clone()) {
                        get_announced(conn, requester, id).await.unwrap_or(None)
                    } else {
                        None
                    };

                let announces: Option<Vec<ApActorTerse>> = if let Some(id) = note.id.clone() {
                    get_announcers(conn, None, None, None, id)
                        .await
                        .ok()
                        .map(|x| x.into_iter().collect())
                } else {
                    None
                };

                let liked = if let (Some(requester), Some(id)) = (requester, note.id.clone()) {
                    get_liked(conn, requester, id).await.unwrap_or(None)
                } else {
                    None
                };

                let likes: Option<Vec<ApActorTerse>> = if let Some(id) = note.id.clone() {
                    get_likers(conn, None, None, None, id)
                        .await
                        .ok()
                        .map(|x| x.into_iter().collect())
                } else {
                    None
                };

                note.ephemeral = Some(Ephemeral {
                    liked,
                    likes,
                    announced,
                    announces,
                    attributed_to,
                    ..note.ephemeral.clone().unwrap_or(Default::default())
                });
                ApObject::Note(note.clone())
            }
            ApObject::Article(ref mut article) => {
                let attributed_to = if let Ok(actor) = retriever::get_actor(
                    conn,
                    article.attributed_to.clone().to_string(),
                    None,
                    true,
                )
                .await
                {
                    Some(vec![ApActorTerse::from(actor)])
                } else {
                    None
                };

                let announced =
                    if let (Some(requester), Some(id)) = (requester.clone(), article.id.clone()) {
                        get_announced(conn, requester, id).await.unwrap_or(None)
                    } else {
                        None
                    };

                let announces: Option<Vec<ApActorTerse>> = if let Some(id) = article.id.clone() {
                    get_announcers(conn, None, None, None, id)
                        .await
                        .ok()
                        .map(|x| x.into_iter().collect())
                } else {
                    None
                };

                let liked = if let (Some(requester), Some(id)) = (requester, article.id.clone()) {
                    get_liked(conn, requester, id).await.unwrap_or(None)
                } else {
                    None
                };

                let likes: Option<Vec<ApActorTerse>> = if let Some(id) = article.id.clone() {
                    get_likers(conn, None, None, None, id)
                        .await
                        .ok()
                        .map(|x| x.into_iter().collect())
                } else {
                    None
                };

                article.ephemeral = Some(Ephemeral {
                    liked,
                    likes,
                    announced,
                    announces,
                    attributed_to,
                    ..article.ephemeral.clone().unwrap_or(Default::default())
                });
                ApObject::Article(article.clone())
            }
            ApObject::Question(ref mut question) => {
                let attributed_to = if let Ok(actor) = retriever::get_actor(
                    conn,
                    question.attributed_to.clone().to_string(),
                    None,
                    true,
                )
                .await
                {
                    Some(vec![ApActorTerse::from(actor)])
                } else {
                    None
                };

                let announced =
                    if let (Some(requester), Some(id)) = (requester.clone(), question.id.clone()) {
                        get_announced(conn, requester, id).await.unwrap_or(None)
                    } else {
                        None
                    };

                let announces: Option<Vec<ApActorTerse>> = if let Some(id) = question.id.clone() {
                    get_announcers(conn, None, None, None, id)
                        .await
                        .ok()
                        .map(|x| x.into_iter().collect())
                } else {
                    None
                };

                let liked =
                    if let (Some(requester), Some(id)) = (requester.clone(), question.id.clone()) {
                        get_liked(conn, requester, id).await.unwrap_or(None)
                    } else {
                        None
                    };

                let likes: Option<Vec<ApActorTerse>> = if let Some(id) = question.id.clone() {
                    get_likers(conn, None, None, None, id)
                        .await
                        .ok()
                        .map(|x| x.into_iter().collect())
                } else {
                    None
                };

                question.ephemeral = Some(Ephemeral {
                    liked,
                    likes,
                    announced,
                    announces,
                    attributed_to,
                    ..question.ephemeral.clone().unwrap_or(Default::default())
                });
                ApObject::Question(question.clone())
            }
            _ => self.clone(),
        }
    }
}
