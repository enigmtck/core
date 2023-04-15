#[macro_use]
extern crate log;

use diesel::prelude::*;
use enigmatick::{
    activity_pub::{
        sender::send_activity, ApAccept, ApActivity, ApActor, ApAddress, ApAnnounce, ApFollow,
        ApLike, ApNote, Metadata,
    },
    models::{
        announces::Announce,
        followers::{Follower, NewFollower},
        likes::Like,
        remote_activities::{NewRemoteActivity, RemoteActivity},
        remote_announces::RemoteAnnounce,
    },
    runner::{
        activity::get_remote_activity_by_apid,
        actor::{get_actor, get_remote_actor_by_ap_id},
        encrypted::{process_join, provide_one_time_key, send_kexinit},
        follow::*,
        note::{
            delete_note, fetch_remote_note, get_links, process_outbound_note, process_remote_note,
            retrieve_context,
        },
        send_to_mq,
        timeline::{
            add_to_timeline, create_timeline_item, get_timeline_item_by_ap_id,
            update_timeline_record,
        },
        user::{get_follower_inboxes, get_profile, get_profile_by_ap_id},
        POOL,
    },
    schema::{announces, followers, likes, remote_activities, remote_announces},
    signing::{Method, SignParams},
    MaybeMultiple, MaybeReference,
};
use faktory::{ConsumerBuilder, Job};
use reqwest::Client;
use std::io;
use tokio::runtime::Runtime;
use webpage::{Webpage, WebpageOptions};

pub fn create_remote_activity(remote_activity: NewRemoteActivity) -> Option<RemoteActivity> {
    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(remote_activities::table)
            .values(&remote_activity)
            .get_result::<RemoteActivity>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
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

pub fn get_remote_announce_by_ap_id(ap_id: String) -> Option<RemoteAnnounce> {
    use enigmatick::schema::remote_announces::dsl::{ap_id as a, remote_announces};

    if let Ok(conn) = POOL.get() {
        match remote_announces
            .filter(a.eq(ap_id))
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

pub fn create_follower(follower: NewFollower) -> Option<Follower> {
    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(followers::table)
            .values(&follower)
            .get_result::<Follower>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

fn acknowledge_followers(job: Job) -> io::Result<()> {
    debug!("running acknowledge job");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    debug!("PROCESSING INCOMING ACCEPT REQUEST");

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();
        log::debug!("APID: {ap_id}");

        handle.block_on(async {
            if let Some(activity) = get_remote_activity_by_apid(ap_id) {
                if let Ok(follow) = ApFollow::try_from(ApActivity::from(activity)) {
                    if let Ok(accept) = ApAccept::try_from(follow.clone()) {
                        if let Some(profile) = get_profile_by_ap_id(accept.actor.clone()) {
                            if let Some((actor, _)) =
                                get_actor(profile.clone(), follow.actor.clone()).await
                            {
                                let inbox = actor.inbox;
                                let activity: ApActivity = accept.clone().into();

                                match send_activity(activity, profile.clone(), inbox.clone()).await
                                {
                                    Ok(_) => {
                                        info!("ACCEPT SENT: {inbox:#?}");

                                        if let Ok(mut follower) = NewFollower::try_from(follow) {
                                            follower.link(profile.clone());

                                            debug!("NEW FOLLOWER\n{follower:#?}");
                                            if create_follower(follower).is_some() {
                                                info!("FOLLOWER CREATED");
                                            }
                                        }
                                    }
                                    Err(e) => error!("ERROR SENDING UNDO REQUEST: {e:#?}"),
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    Ok(())
}

fn process_announce(job: Job) -> io::Result<()> {
    debug!("running process_announce job");

    let ap_ids = job.args();

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for ap_id in ap_ids {
        let ap_id = ap_id.as_str().unwrap().to_string();
        debug!("looking for ap_id: {}", ap_id);

        let announce = get_remote_announce_by_ap_id(ap_id);

        if let Some(announce) = announce {
            let activity: ApActivity = announce.clone().into();

            if activity.kind == "Announce".into() {
                if let MaybeReference::Reference(note_id) = activity.clone().object {
                    if get_timeline_item_by_ap_id(note_id.clone()).is_none() {
                        handle.block_on(async {
                            let note = fetch_remote_note(note_id.clone()).await;

                            if let Some(ap_note) = note {
                                if let Some(timeline_item) =
                                    create_timeline_item((activity, ap_note.clone()).into())
                                {
                                    add_to_timeline(
                                        Option::from(
                                            serde_json::to_value(ap_note.clone().to).unwrap(),
                                        ),
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
                    link_remote_announces_to_timeline(note_id);
                }
            }
        }
    }

    Ok(())
}

fn send_like(job: Job) -> io::Result<()> {
    debug!("SENDING LIKE");

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

                        let signature = enigmatick::signing::sign(SignParams {
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
                                    debug!("SEND SUCCESSFUL: {url}\n{text}");
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

fn send_announce(job: Job) -> io::Result<()> {
    debug!("SENDING ANNOUNCE");

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

                                    let signature = enigmatick::signing::sign(SignParams {
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
                                                debug!("SEND SUCCESSFUL: {url}\n{text}");
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

fn main() {
    env_logger::init();

    let faktory_url = &*enigmatick::FAKTORY_URL;

    info!("starting faktory consumer: {}", faktory_url);

    let mut consumer = ConsumerBuilder::default();
    consumer.register("acknowledge_followers", acknowledge_followers);
    consumer.register("provide_one_time_key", provide_one_time_key);
    consumer.register("process_remote_note", process_remote_note);
    consumer.register("process_join", process_join);
    consumer.register("process_outbound_note", process_outbound_note);
    consumer.register("process_announce", process_announce);
    consumer.register("send_kexinit", send_kexinit);
    consumer.register("update_timeline_record", update_timeline_record);
    consumer.register("retrieve_context", retrieve_context);
    consumer.register("send_like", send_like);
    consumer.register("send_announce", send_announce);
    consumer.register("delete_note", delete_note);
    consumer.register("process_follow", process_follow);
    consumer.register("process_accept", process_accept);
    consumer.register("process_undo_follow", process_undo_follow);
    consumer.register("process_remote_undo_follow", process_remote_undo_follow);

    consumer.register("test_job", |job| {
        debug!("{:#?}", job);
        Ok(())
    });

    let mut consumer = consumer.connect(Some(faktory_url)).unwrap();

    if let Err(e) = consumer.run(&["default"]) {
        error!("worker failed: {}", e);
    }
}
