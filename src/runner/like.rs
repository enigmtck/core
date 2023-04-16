use diesel::prelude::*;
use faktory::Job;
use reqwest::Client;
use std::io;
use tokio::runtime::Runtime;

use crate::{
    activity_pub::ApLike,
    models::likes::Like,
    runner::{actor::get_remote_actor_by_ap_id, user::get_profile},
    schema::likes,
    signing::{Method, SignParams},
};

use super::POOL;

pub fn get_like_by_uuid(uuid: String) -> Option<Like> {
    if let Ok(conn) = POOL.get() {
        match likes::table
            .filter(likes::uuid.eq(uuid))
            .first::<Like>(&conn)
        {
            Ok(x) => Option::from(x),
            Err(_) => Option::None,
        }
    } else {
        None
    }
}

pub fn send_like(job: Job) -> io::Result<()> {
    log::debug!("SENDING LIKE");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for uuid in job.args() {
        if let Some(like) = get_like_by_uuid(uuid.as_str().unwrap().to_string()) {
            if let Some(profile_id) = like.profile_id {
                if let Some(sender) = get_profile(profile_id) {
                    if let Some(actor) = get_remote_actor_by_ap_id(like.ap_to.clone()) {
                        let ap_like = ApLike::from(like.clone());
                        let url = actor.inbox;
                        let body = Option::from(serde_json::to_string(&ap_like).unwrap());
                        let method = Method::Post;

                        let signature = crate::signing::sign(SignParams {
                            profile: sender.clone(),
                            url: url.clone(),
                            body: body.clone(),
                            method,
                        });

                        let client = Client::new()
                            .post(&url)
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
                                if let Ok(text) = resp.text().await {
                                    log::debug!("SEND SUCCESSFUL: {url}\n{text}");
                                }
                            }
                        })
                    }
                }
            }
        }
    }

    Ok(())
}
