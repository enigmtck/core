use std::collections::HashSet;

use diesel::{r2d2::ConnectionManager, PgConnection};
use lapin::{options::BasicPublishOptions, BasicProperties, ConnectionProperties};
use lazy_static::lazy_static;
use reqwest::Client;
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{ApNote, ApObject},
    models::profiles::Profile,
    signing::{Method, SignParams},
};

pub mod activity;
pub mod actor;
pub mod announce;
pub mod encrypted;
pub mod follow;
pub mod like;
pub mod note;
pub mod processing;
pub mod timeline;
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

pub fn send_to_inboxes(inboxes: HashSet<String>, profile: Profile, message: ApObject) {
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for url in inboxes {
        let body = Option::from(serde_json::to_string(&message).unwrap());
        let method = Method::Post;

        let signature = crate::signing::sign(SignParams {
            profile: profile.clone(),
            url: url.clone(),
            body: body.clone(),
            method,
        });

        let client = Client::new()
            .post(url.clone())
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
