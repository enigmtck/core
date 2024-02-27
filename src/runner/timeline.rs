use diesel::prelude::*;
use faktory::Job;
use serde_json::Value;
use std::io;
use tokio::runtime::Runtime;

use crate::{
    models::{
        remote_actors::{get_follower_profiles_by_endpoint, get_leader_by_endpoint},
        remote_notes::RemoteNote,
        timeline::{
            create_timeline_item_cc, create_timeline_item_to, update_timeline_items, TimelineItem,
        },
    },
    schema::remote_notes,
    POOL,
};

pub fn update_timeline_record(job: Job) -> io::Result<()> {
    log::debug!("running update_timeline_record job");

    let ap_ids = job.args();

    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    match POOL.get() {
        Ok(mut conn) => {
            for ap_id in ap_ids {
                let ap_id = ap_id.as_str().unwrap().to_string();
                log::debug!("looking for ap_id: {}", ap_id);

                match remote_notes::table
                    .filter(remote_notes::ap_id.eq(ap_id))
                    .first::<RemoteNote>(&mut conn)
                {
                    Ok(remote_note) => {
                        if remote_note.kind == "Note" {
                            handle.block_on(async {
                                update_timeline_items(
                                    POOL.get()
                                        .expect("failed to get database connection")
                                        .into(),
                                    (None, remote_note.clone().into()).into(),
                                )
                                .await
                            });
                        }
                    }
                    Err(e) => log::error!("error: {:#?}", e),
                }
            }
        }
        Err(e) => log::error!("error: {:#?}", e),
    }

    Ok(())
}

async fn add_timeline_item_to_for_recipient(recipient: &str, timeline_item: &TimelineItem) {
    if create_timeline_item_to(
        POOL.get()
            .expect("failed to get database connection")
            .into(),
        (timeline_item.clone(), recipient.to_string()),
    )
    .await
        && get_leader_by_endpoint(
            POOL.get()
                .expect("failed to get database connection")
                .into(),
            recipient.to_string(),
        )
        .await
        .is_some()
    {
        for (_remote_actor, _leader, profile) in get_follower_profiles_by_endpoint(
            POOL.get()
                .expect("failed to get database connection")
                .into(),
            recipient.to_string(),
        )
        .await
        {
            if let Some(follower) = profile {
                let follower_endpoint =
                    format!("{}/user/{}", &*crate::SERVER_URL, follower.username);
                create_timeline_item_to(
                    POOL.get()
                        .expect("failed to get database connection")
                        .into(),
                    (timeline_item.clone(), follower_endpoint),
                )
                .await;
            }
        }
    }
}

async fn add_timeline_item_cc_for_recipient(recipient: &str, timeline_item: &TimelineItem) {
    if create_timeline_item_cc(
        POOL.get()
            .expect("failed to get database connection")
            .into(),
        (timeline_item.clone(), recipient.to_string()),
    )
    .await
        && get_leader_by_endpoint(
            POOL.get()
                .expect("failed to get database connection")
                .into(),
            recipient.to_string(),
        )
        .await
        .is_some()
    {
        for (_remote_actor, _leader, profile) in get_follower_profiles_by_endpoint(
            POOL.get()
                .expect("failed to get database connection")
                .into(),
            recipient.to_string(),
        )
        .await
        {
            if let Some(follower) = profile {
                let follower_endpoint =
                    format!("{}/user/{}", &*crate::SERVER_URL, follower.username);
                create_timeline_item_cc(
                    POOL.get()
                        .expect("failed to get database connection")
                        .into(),
                    (timeline_item.clone(), follower_endpoint),
                )
                .await;
            }
        }
    }
}

pub async fn add_to_timeline(ap_to: Option<Value>, cc: Option<Value>, timeline_item: TimelineItem) {
    if let Some(ap_to) = ap_to {
        if let Ok(to_vec) = serde_json::from_value::<Vec<String>>(ap_to.clone()) {
            for to in to_vec {
                log::debug!("ADDING TO FOR {to}");
                add_timeline_item_to_for_recipient(&to, &timeline_item).await;
            }
        } else {
            log::error!("TO VALUE NOT A VEC: {ap_to:#?}");
        }
    }

    if let Some(cc) = cc {
        if let Ok(cc_vec) = serde_json::from_value::<Vec<String>>(cc.clone()) {
            for cc in cc_vec {
                log::debug!("ADDING CC FOR {cc}");
                add_timeline_item_cc_for_recipient(&cc, &timeline_item).await;
            }
        } else {
            log::error!("CC VALUE NOT A VEC: {cc:#?}");
        }
    };
}
