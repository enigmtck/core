use std::collections::HashSet;

use diesel::{r2d2::ConnectionManager, PgConnection};
use lapin::{options::BasicPublishOptions, BasicProperties, ConnectionProperties};
use lazy_static::lazy_static;
use reqwest::Client;
use tokio::runtime::Runtime;
use url::Url;

use crate::{
    activity_pub::{ApActivity, ApActor, ApAddress, ApNote},
    models::profiles::Profile,
    signing::{Method, SignParams},
    MaybeMultiple, MaybeReference,
};

use self::{actor::get_actor, user::get_follower_inboxes};

pub mod activity;
pub mod actor;
pub mod announce;
pub mod cache;
pub mod encrypted;
pub mod follow;
pub mod like;
pub mod note;
pub mod processing;
pub mod timeline;
pub mod undo;
pub mod user;

type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

lazy_static! {
    pub static ref POOL: Pool = {
        let database_url = &*crate::DATABASE_URL;
        log::debug!("database: {}", database_url);
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        Pool::new(manager).expect("failed to create db pool")
    };
}

pub fn clean_text(text: String) -> String {
    let ammonia = ammonia::Builder::default();

    ammonia.clean(&text).to_string()
}

pub async fn send_to_mq(note: ApNote) {
    let mq = lapin::Connection::connect(&crate::AMQP_URL, ConnectionProperties::default())
        .await
        .unwrap();
    log::debug!("SENDING TO MQ");

    let channel = mq.create_channel().await.unwrap();
    // let _queue = channel
    //     .queue_declare(
    //         "events",
    //         QueueDeclareOptions::default(),
    //         FieldTable::default(),
    //     )
    //     .await
    //     .unwrap();

    let _confirm = channel
        .basic_publish(
            "",
            "events",
            BasicPublishOptions::default(),
            &serde_json::to_vec(&note).unwrap(),
            BasicProperties::default(),
        )
        .await
        .unwrap()
        .await
        .unwrap();
}

pub fn send_to_inboxes(inboxes: Vec<ApAddress>, profile: Profile, message: ApActivity) {
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    log::debug!("INBOXES\n{inboxes:#?}");

    for url_str in inboxes {
        log::debug!("SENDING TO {url_str}");

        let body = Option::from(serde_json::to_string(&message).unwrap());
        let method = Method::Post;

        if let Ok(url) = Url::parse(&url_str.clone().to_string()) {
            if let Ok(signature) = crate::signing::sign(SignParams {
                profile: profile.clone(),
                url: url.clone(),
                body: body.clone(),
                method,
            }) {
                let client = Client::new()
                    .post(url_str.clone().to_string())
                    .header("Date", signature.date)
                    .header("Digest", signature.digest.unwrap())
                    .header("Signature", &signature.signature)
                    .header(
                        "Content-Type",
                        "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"",
                    )
                    .body(body.unwrap());

                handle.block_on(async {
                    if let Ok(resp) = client.send().await {
                        let code = resp.status();
                        log::debug!("SEND RESULT FOR {url}: {code}");
                    }
                });
            }
        }
    }
}

fn handle_recipients(inboxes: &mut HashSet<ApAddress>, sender: &Profile, address: &ApAddress) {
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    let actor = ApActor::from(sender.clone());

    if address.is_public() {
        inboxes.extend(get_follower_inboxes(sender.clone()));
        // instead of the above, consider sending to shared inboxes of known instances
        // the duplicate code is temporary because some operations (e.g., Delete) do not have
        // the followers in cc, so until there's logic to send more broadly to all instances,
        // this will need to suffice
    } else if let Some(followers) = actor.followers {
        if address.to_string() == followers {
            inboxes.extend(get_follower_inboxes(sender.clone()));
        } else if let Some((actor, _)) = handle
            .block_on(async { get_actor(Some(sender.clone()), address.clone().to_string()).await })
        {
            inboxes.insert(ApAddress::Address(actor.inbox));
        }
    }
}

pub fn get_inboxes(activity: ApActivity, sender: Profile) -> Vec<ApAddress> {
    let mut inboxes = HashSet::<ApAddress>::new();

    let (to, cc) = match activity {
        ApActivity::Create(activity) => (Some(activity.to), activity.cc),
        ApActivity::Delete(activity) => (Some(activity.to), None),
        ApActivity::Announce(activity) => (Some(activity.to), activity.cc),
        ApActivity::Like(activity) => (activity.to, None),
        ApActivity::Follow(activity) => {
            if let MaybeReference::Reference(id) = activity.object {
                (Some(MaybeMultiple::Single(ApAddress::Address(id))), None)
            } else {
                (None, None)
            }
        }
        ApActivity::Undo(activity) => {
            if let MaybeReference::Actual(target_activity) = activity.object {
                match target_activity {
                    ApActivity::Follow(follow) => {
                        if let MaybeReference::Reference(target) = follow.object {
                            (
                                Some(MaybeMultiple::Single(ApAddress::Address(target))),
                                None,
                            )
                        } else {
                            (None, None)
                        }
                    }
                    ApActivity::Like(like) => (like.to, None),
                    ApActivity::Announce(announce) => (
                        announce.cc,
                        Some(MaybeMultiple::Single(ApAddress::get_public())),
                    ),
                    _ => (None, None),
                }
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    };

    if let Some(to) = to {
        match to {
            MaybeMultiple::Single(to) => {
                handle_recipients(&mut inboxes, &sender, &to);
            }
            MaybeMultiple::Multiple(to) => {
                for address in to.iter() {
                    handle_recipients(&mut inboxes, &sender, address);
                }
            }
            MaybeMultiple::None => {}
        }
    }

    if let Some(cc) = cc {
        match cc {
            MaybeMultiple::Single(to) => {
                handle_recipients(&mut inboxes, &sender, &to);
            }
            MaybeMultiple::Multiple(to) => {
                for address in to.iter() {
                    handle_recipients(&mut inboxes, &sender, address);
                }
            }
            MaybeMultiple::None => {}
        }
    }

    inboxes.into_iter().collect()
}
