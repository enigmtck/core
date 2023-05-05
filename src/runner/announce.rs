use diesel::prelude::*;
use faktory::Job;
use std::io;
use tokio::runtime::Runtime;
use webpage::{Webpage, WebpageOptions};

use crate::{
    activity_pub::{ApActivity, ApAddress, ApAnnounce, ApNote, Metadata},
    models::remote_announces::RemoteAnnounce,
    runner::{
        activity::get_activity_by_uuid,
        get_inboxes,
        note::{fetch_remote_note, get_links},
        send_to_inboxes, send_to_mq,
        timeline::{add_to_timeline, create_timeline_item, get_timeline_item_by_ap_id},
        user::get_profile,
    },
    schema::remote_announces,
};

use super::POOL;

pub fn send_announce(job: Job) -> io::Result<()> {
    log::debug!("SENDING ANNOUNCE");

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();
        log::debug!("LOOKING FOR UUID {uuid}");

        if let Some((
            activity,
            target_note,
            target_remote_note,
            target_profile,
            target_remote_actor,
        )) = get_activity_by_uuid(uuid.clone())
        {
            log::debug!("FOUND ACTIVITY\n{activity:#?}");
            if let Some(sender) = get_profile(activity.profile_id) {
                if let Ok(activity) = ApActivity::try_from((
                    (
                        activity,
                        target_note,
                        target_remote_note,
                        target_profile,
                        target_remote_actor,
                    ),
                    None,
                )) {
                    let inboxes: Vec<ApAddress> = get_inboxes(activity.clone(), sender.clone());
                    send_to_inboxes(inboxes, sender, activity.clone());
                }
            }
        }
    }

    Ok(())
}

pub fn process_announce(job: Job) -> io::Result<()> {
    log::debug!("running process_announce job");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();
        log::debug!("looking for ap_id: {}", ap_id);

        let announce = get_remote_announce_by_ap_id(ap_id);

        if let Some(announce) = announce {
            if let Ok(activity) = ApAnnounce::try_from(announce.clone()) {
                if get_timeline_item_by_ap_id(activity.object.clone().to_string()).is_none() {
                    handle.block_on(async {
                        let note = fetch_remote_note(activity.object.clone().to_string()).await;

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
                link_remote_announces_to_timeline(activity.object.clone().to_string());
            }
        }
    }

    Ok(())
}

pub fn get_remote_announce_by_ap_id(ap_id: String) -> Option<RemoteAnnounce> {
    if let Ok(mut conn) = POOL.get() {
        match remote_announces::table
            .filter(remote_announces::ap_id.eq(ap_id))
            .first::<RemoteAnnounce>(&mut conn)
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

        if let Ok(mut conn) = POOL.get() {
            if let Ok(x) = diesel::update(
                remote_announces::table.filter(remote_announces::ap_object.eq(timeline_ap_id)),
            )
            .set(remote_announces::timeline_id.eq(timeline.id))
            .execute(&mut conn)
            {
                log::debug!("{x} ANNOUNCE ROWS UPDATED");
            }
        }
    }
}
