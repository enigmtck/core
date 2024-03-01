use std::collections::HashSet;

use diesel::{r2d2::ConnectionManager, PgConnection};
use faktory::ConsumerBuilder;
use lapin::{options::BasicPublishOptions, BasicProperties, ConnectionProperties};
use reqwest::Client;
use url::Url;

use crate::{
    activity_pub::{ApActivity, ApActor, ApAddress, ApNote},
    models::profiles::Profile,
    runner::{
        announce::{process_remote_announce, process_remote_undo_announce, send_announce},
        encrypted::{process_join, provide_one_time_key, send_kexinit},
        follow::{
            acknowledge_followers, process_accept, process_follow, process_remote_undo_follow,
        },
        like::{process_remote_undo_like, send_like},
        note::{delete_note, process_outbound_note, process_remote_note, retrieve_context},
        timeline::update_timeline_record,
        undo::process_outbound_undo,
        user::send_profile_update,
    },
    signing::{Method, SignParams},
    MaybeMultiple, MaybeReference,
};

use self::{actor::get_actor, user::get_follower_inboxes};

pub mod actor;
pub mod announce;
pub mod cache;
pub mod encrypted;
pub mod follow;
pub mod like;
pub mod note;
pub mod timeline;
pub mod undo;
pub mod user;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

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

pub async fn send_to_inboxes(inboxes: Vec<ApAddress>, profile: Profile, message: ApActivity) {
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
                    .header("Content-Type", "application/activity+json")
                    .body(body.clone().unwrap());

                log::debug!("{client:#?}");
                log::debug!("{body:#?}");

                if let Ok(resp) = client.send().await {
                    let code = resp.status();
                    let message = resp.text().await;
                    log::debug!("SEND RESULT FOR {url}: {code} {message:#?}");
                }
            }
        }
    }
}

async fn handle_recipients(
    inboxes: &mut HashSet<ApAddress>,
    sender: &Profile,
    address: &ApAddress,
) {
    let actor = ApActor::from(sender.clone());

    if address.is_public() {
        inboxes.extend(get_follower_inboxes(sender.clone()).await);
        // instead of the above, consider sending to shared inboxes of known instances
        // the duplicate code is temporary because some operations (e.g., Delete) do not have
        // the followers in cc, so until there's logic to send more broadly to all instances,
        // this will need to suffice
    } else if let Some(followers) = actor.followers {
        if address.to_string() == followers {
            inboxes.extend(get_follower_inboxes(sender.clone()).await);
        } else if let Some((actor, _)) =
            get_actor(sender.clone(), address.clone().to_string()).await
        {
            inboxes.insert(ApAddress::Address(actor.inbox));
        }
    }
}

pub async fn get_inboxes(activity: ApActivity, sender: Profile) -> Vec<ApAddress> {
    let mut inboxes = HashSet::<ApAddress>::new();

    let (to, cc) = match activity {
        ApActivity::Create(activity) => (Some(activity.to), activity.cc),
        ApActivity::Delete(activity) => (Some(activity.to), activity.cc),
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

    let consolidated = match (to, cc) {
        (Some(to), Some(MaybeMultiple::Multiple(cc))) => to.extend(cc),
        (Some(to), Some(MaybeMultiple::Single(cc))) => to.extend(vec![cc]),
        (Some(to), Some(MaybeMultiple::None)) => to,
        (Some(to), None) => to,
        (None, Some(cc)) => cc,
        (None, None) => MaybeMultiple::None,
    };

    match consolidated {
        MaybeMultiple::Single(to) => {
            handle_recipients(&mut inboxes, &sender, &to).await;
        }
        MaybeMultiple::Multiple(to) => {
            for address in to.iter() {
                handle_recipients(&mut inboxes, &sender, address).await;
            }
        }
        MaybeMultiple::None => {}
    }

    inboxes.into_iter().collect()
}

pub fn start() {
    env_logger::init();

    let faktory_url = &*crate::FAKTORY_URL;

    log::info!("STARTING FAKTORY CONSUMER: {}", faktory_url);

    let mut consumer = ConsumerBuilder::default();
    consumer.register("acknowledge_followers", acknowledge_followers);
    consumer.register("provide_one_time_key", provide_one_time_key);
    consumer.register("process_remote_note", process_remote_note);
    consumer.register("process_join", process_join);
    consumer.register("process_outbound_note", process_outbound_note);
    consumer.register("process_remote_announce", process_remote_announce);
    consumer.register("send_kexinit", send_kexinit);
    consumer.register("update_timeline_record", update_timeline_record);
    consumer.register("retrieve_context", retrieve_context);
    consumer.register("send_like", send_like);
    consumer.register("send_announce", send_announce);
    consumer.register("delete_note", delete_note);
    consumer.register("process_follow", process_follow);
    consumer.register("process_accept", process_accept);
    consumer.register("process_remote_undo_follow", process_remote_undo_follow);
    consumer.register("send_profile_update", send_profile_update);
    consumer.register("process_undo", process_outbound_undo);
    consumer.register("process_remote_undo_announce", process_remote_undo_announce);
    consumer.register("process_remote_undo_like", process_remote_undo_like);

    let mut consumer = consumer.connect(Some(faktory_url)).unwrap();

    if let Err(e) = consumer.run(&["default"]) {
        log::error!("worker failed: {}", e);
    }
}
