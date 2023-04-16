use diesel::prelude::*;
use faktory::Job;
use reqwest::Client;
use std::io;
use tokio::runtime::Runtime;
use webpage::{Webpage, WebpageOptions};

use crate::{
    activity_pub::{ApActivity, ApActor, ApAddress, ApAnnounce, ApNote, Metadata},
    models::{announces::Announce, remote_announces::RemoteAnnounce},
    runner::{
        actor::get_actor,
        note::{fetch_remote_note, get_links},
        send_to_mq,
        timeline::{add_to_timeline, create_timeline_item, get_timeline_item_by_ap_id},
        user::{get_follower_inboxes, get_profile},
    },
    schema::{announces, remote_announces},
    signing::{Method, SignParams},
    MaybeMultiple, MaybeReference,
};

use super::POOL;

pub fn send_announce(job: Job) -> io::Result<()> {
    log::debug!("SENDING ANNOUNCE");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for uuid in job.args() {
        if let Some(mut announce) = get_announce_by_uuid(uuid.as_str().unwrap().to_string()) {
            if let Some(profile_id) = announce.profile_id {
                if let Some(sender) = get_profile(profile_id) {
                    if let Ok(ap_to) =
                        serde_json::from_value::<MaybeMultiple<ApAddress>>(announce.ap_to.clone())
                    {
                        if let Some(address) = ap_to.single() {
                            if address.is_public() {
                                let mut inboxes = get_follower_inboxes(sender.clone());

                                // we assume coming in to this that we only have one address in the
                                // cc field, belonging to the original sender of the announced post;
                                // before inserting the followers url in to the cc vec, we grab that
                                // single sender address to add to the inboxes set for direct delivey.
                                // this is ugly, but it saves us from having to figure out if the
                                // addresses in that vec are really addresses or weird "followers" URLs.
                                // will probably need to rethink this later

                                let actor: ApActor = sender.clone().into();

                                if let (Some(followers), Some(cc)) =
                                    (actor.followers, announce.cc.clone())
                                {
                                    if let Ok(cc) =
                                        serde_json::from_value::<MaybeMultiple<ApAddress>>(cc)
                                    {
                                        announce.cc = match cc {
                                            MaybeMultiple::Multiple(mut cc) => {
                                                handle.block_on(async {
                                                    let original_sender = get_actor(
                                                        sender.clone(),
                                                        cc[0].to_string(),
                                                    )
                                                    .await;

                                                    if let Some((original_sender, _leader)) =
                                                        original_sender
                                                    {
                                                        inboxes.insert(original_sender.inbox);
                                                    }
                                                });

                                                cc.push(ApAddress::Address(followers));
                                                Some(
                                                    serde_json::to_value(MaybeMultiple::Multiple(
                                                        cc,
                                                    ))
                                                    .unwrap(),
                                                )
                                            }
                                            MaybeMultiple::Single(cc) => {
                                                handle.block_on(async {
                                                    let original_sender =
                                                        get_actor(sender.clone(), cc.to_string())
                                                            .await;

                                                    if let Some((original_sender, _leader)) =
                                                        original_sender
                                                    {
                                                        inboxes.insert(original_sender.inbox);
                                                    }
                                                });
                                                Some(
                                                    serde_json::to_value(MaybeMultiple::Multiple(
                                                        vec![cc, ApAddress::Address(followers)],
                                                    ))
                                                    .unwrap(),
                                                )
                                            }
                                            MaybeMultiple::None => None,
                                        };
                                    }
                                }

                                let ap_announce = ApAnnounce::from(announce.clone());

                                log::debug!("SENDING ANNOUNCE\n{ap_announce:#?}");

                                for url in inboxes {
                                    let body =
                                        Option::from(serde_json::to_string(&ap_announce).unwrap());
                                    let method = Method::Post;

                                    let signature = crate::signing::sign(SignParams {
                                        profile: sender.clone(),
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
            }
        }
    }

    Ok(())
}

pub fn process_announce(job: Job) -> io::Result<()> {
    log::debug!("running process_announce job");

    let ap_ids = job.args();

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for ap_id in ap_ids {
        let ap_id = ap_id.as_str().unwrap().to_string();
        log::debug!("looking for ap_id: {}", ap_id);

        let announce = get_remote_announce_by_ap_id(ap_id);

        if let Some(announce) = announce {
            if let Ok(activity) = ApAnnounce::try_from(announce.clone()) {
                if get_timeline_item_by_ap_id(activity.object.clone()).is_none() {
                    handle.block_on(async {
                        let note = fetch_remote_note(activity.object.clone()).await;

                        if let Some(ap_note) = note {
                            if let Some(timeline_item) =
                                create_timeline_item((activity.clone(), ap_note.clone()).into())
                            {
                                add_to_timeline(
                                    Option::from(serde_json::to_value(ap_note.clone().to).unwrap()),
                                    {
                                        if let Some(cc) = ap_note.clone().cc {
                                            Option::from(serde_json::to_value(cc).unwrap())
                                        } else {
                                            Option::None
                                        }
                                    },
                                    timeline_item.clone(),
                                );

                                let mut ap_note: ApNote = timeline_item.into();
                                let links = get_links(ap_note.content.clone());

                                let metadata: Vec<Metadata> = {
                                    links
                                        .iter()
                                        .map(|link| {
                                            Webpage::from_url(link, WebpageOptions::default())
                                        })
                                        .filter(|metadata| metadata.is_ok())
                                        .map(|metadata| metadata.unwrap().html.meta.into())
                                        .collect()
                                };
                                ap_note.ephemeral_metadata = Some(metadata);
                                send_to_mq(ap_note).await;
                            }
                        }
                    });
                }

                // TODO: also update the updated_at time on timeline and surface that in the
                // client to bring bump it in the view
                link_remote_announces_to_timeline(activity.object.clone());
            }
        }
    }

    Ok(())
}

pub fn get_remote_announce_by_ap_id(ap_id: String) -> Option<RemoteAnnounce> {
    if let Ok(conn) = POOL.get() {
        match remote_announces::table
            .filter(remote_announces::ap_id.eq(ap_id))
            .first::<RemoteAnnounce>(&conn)
        {
            Ok(x) => Option::from(x),
            Err(_) => Option::None,
        }
    } else {
        Option::None
    }
}

pub fn link_remote_announces_to_timeline(timeline_ap_id: String) {
    if let Some(timeline) = get_timeline_item_by_ap_id(timeline_ap_id) {
        let timeline_ap_id = serde_json::to_value(timeline.ap_id).unwrap();

        if let Ok(conn) = POOL.get() {
            if let Ok(x) = diesel::update(
                remote_announces::table.filter(remote_announces::ap_object.eq(timeline_ap_id)),
            )
            .set(remote_announces::timeline_id.eq(timeline.id))
            .execute(&conn)
            {
                log::debug!("{x} ANNOUNCE ROWS UPDATED");
            }
        }
    }
}

pub fn get_announce_by_uuid(uuid: String) -> Option<Announce> {
    if let Ok(conn) = POOL.get() {
        match announces::table
            .filter(announces::uuid.eq(uuid))
            .first::<Announce>(&conn)
        {
            Ok(x) => Option::from(x),
            Err(_) => Option::None,
        }
    } else {
        None
    }
}
