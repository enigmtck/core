use anyhow::Result;

use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::models::{
    remote_actors::{get_follower_profiles_by_endpoint, get_leader_by_endpoint},
    remote_notes::get_remote_note_by_ap_id,
    timeline::{
        create_timeline_item_cc, create_timeline_item_to, update_timeline_items, TimelineItem,
    },
};

use super::TaskError;

pub async fn update_timeline_record_task(
    conn: Option<Db>,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    for ap_id in ap_ids {
        log::debug!("looking for ap_id: {}", ap_id);

        let remote_note = get_remote_note_by_ap_id(conn, ap_id).await;

        if let Some(remote_note) = remote_note {
            if remote_note.kind.as_str() == "note" {
                update_timeline_items(conn, (None, remote_note.clone().into()).into()).await;
            }
        }
    }

    Ok(())
}

async fn add_timeline_item_to_for_recipient(recipient: &str, timeline_item: &TimelineItem) {
    if create_timeline_item_to(None, (timeline_item.clone(), recipient.to_string())).await
        && get_leader_by_endpoint(None, recipient.to_string())
            .await
            .is_some()
    {
        for (_remote_actor, _leader, profile) in
            get_follower_profiles_by_endpoint(None, recipient.to_string()).await
        {
            if let Some(follower) = profile {
                let follower_endpoint =
                    format!("{}/user/{}", &*crate::SERVER_URL, follower.username);
                create_timeline_item_to(None, (timeline_item.clone(), follower_endpoint)).await;
            }
        }
    }
}

async fn add_timeline_item_cc_for_recipient(recipient: &str, timeline_item: &TimelineItem) {
    if create_timeline_item_cc(None, (timeline_item.clone(), recipient.to_string())).await
        && get_leader_by_endpoint(None, recipient.to_string())
            .await
            .is_some()
    {
        for (_remote_actor, _leader, profile) in
            get_follower_profiles_by_endpoint(None, recipient.to_string()).await
        {
            if let Some(follower) = profile {
                let follower_endpoint =
                    format!("{}/user/{}", &*crate::SERVER_URL, follower.username);
                create_timeline_item_cc(None, (timeline_item.clone(), follower_endpoint)).await;
            }
        }
    }
}

pub async fn add_to_timeline(
    ap_to: Option<String>,
    cc: Option<String>,
    timeline_item: TimelineItem,
) {
    if let Some(ap_to) = ap_to {
        if let Ok(to_vec) = serde_json::from_str::<Vec<String>>(&ap_to.clone()) {
            for to in to_vec {
                log::debug!("ADDING TO FOR {to}");
                add_timeline_item_to_for_recipient(&to, &timeline_item).await;
            }
        } else {
            log::error!("TO VALUE NOT A VEC: {ap_to:#?}");
        }
    }

    if let Some(cc) = cc {
        if let Ok(cc_vec) = serde_json::from_str::<Vec<String>>(&cc.clone()) {
            for cc in cc_vec {
                log::debug!("ADDING CC FOR {cc}");
                add_timeline_item_cc_for_recipient(&cc, &timeline_item).await;
            }
        } else {
            log::error!("CC VALUE NOT A VEC: {cc:#?}");
        }
    };
}
