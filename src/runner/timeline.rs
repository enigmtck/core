use diesel::prelude::*;
use faktory::Job;
use serde_json::Value;
use std::io;

use crate::{
    models::{
        remote_notes::RemoteNote,
        timeline::{
            NewTimelineItem, NewTimelineItemCc, NewTimelineItemTo, TimelineItem, TimelineItemCc,
            TimelineItemTo,
        },
    },
    runner::POOL,
    schema::{remote_notes, timeline_to},
    schema::{timeline, timeline_cc},
};

use super::actor::{get_follower_profiles_by_endpoint, get_leader_by_endpoint};

pub fn update_timeline_record(job: Job) -> io::Result<()> {
    log::debug!("running update_timeline_record job");

    let ap_ids = job.args();

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
                            update_timeline_items((None, remote_note.clone().into()).into());
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

fn add_timeline_item_for_recipient<F>(
    recipient: &str,
    timeline_item: &TimelineItem,
    create_directed_timeline_item: F,
) where
    F: Fn((TimelineItem, String)) -> bool,
{
    if create_directed_timeline_item((timeline_item.clone(), recipient.to_string()))
        && get_leader_by_endpoint(recipient.to_string()).is_some()
    {
        for (_remote_actor, _leader, profile) in
            get_follower_profiles_by_endpoint(recipient.to_string())
        {
            if let Some(follower) = profile {
                let follower_endpoint =
                    format!("{}/user/{}", &*crate::SERVER_URL, follower.username);
                create_directed_timeline_item((timeline_item.clone(), follower_endpoint));
            }
        }
    }
}

pub fn add_to_timeline(ap_to: Option<Value>, cc: Option<Value>, timeline_item: TimelineItem) {
    if let Some(ap_to) = ap_to {
        if let Ok(to_vec) = serde_json::from_value::<Vec<String>>(ap_to.clone()) {
            for to in to_vec {
                log::debug!("ADDING TO FOR {to}");
                add_timeline_item_for_recipient(&to, &timeline_item, create_timeline_item_to);
            }
        } else {
            log::error!("TO VALUE NOT A VEC: {ap_to:#?}");
        }
    }

    if let Some(cc) = cc {
        if let Ok(cc_vec) = serde_json::from_value::<Vec<String>>(cc.clone()) {
            for cc in cc_vec {
                log::debug!("ADDING CC FOR {cc}");
                add_timeline_item_for_recipient(&cc, &timeline_item, create_timeline_item_cc);
            }
        } else {
            log::error!("CC VALUE NOT A VEC: {cc:#?}");
        }
    };
}

#[derive(Debug)]
pub enum TimelineDeleteError {
    ConnectionError,
    DatabaseError(diesel::result::Error),
}

pub fn delete_timeline_item_by_ap_id(ap_id: String) -> Result<usize, TimelineDeleteError> {
    match POOL.get() {
        Ok(mut conn) => {
            match diesel::delete(timeline::table.filter(timeline::ap_id.eq(ap_id)))
                .execute(&mut conn)
            {
                Ok(x) => Ok(x),
                Err(e) => {
                    log::error!("FAILED TO DELETE\n{e:#?}");
                    Err(TimelineDeleteError::DatabaseError(e))
                }
            }
        }
        Err(_) => Err(TimelineDeleteError::ConnectionError),
    }
}

pub fn get_timeline_item_by_ap_id(ap_id: String) -> Option<TimelineItem> {
    if let Ok(mut conn) = POOL.get() {
        match timeline::table
            .filter(timeline::ap_id.eq(ap_id))
            .first::<TimelineItem>(&mut conn)
        {
            Ok(x) => Option::from(x),
            Err(_) => Option::None,
        }
    } else {
        Option::None
    }
}

pub fn update_timeline_items(timeline_item: NewTimelineItem) -> Vec<TimelineItem> {
    if let Ok(mut conn) = POOL.get() {
        match diesel::update(timeline::table.filter(timeline::ap_id.eq(timeline_item.ap_id)))
            .set(timeline::content.eq(timeline_item.content))
            .get_results::<TimelineItem>(&mut conn)
        {
            Ok(x) => x,
            Err(_) => {
                vec![]
            }
        }
    } else {
        vec![]
    }
}

pub fn create_timeline_item(timeline_item: NewTimelineItem) -> Option<TimelineItem> {
    if let Ok(mut conn) = POOL.get() {
        match diesel::insert_into(timeline::table)
            .values(&timeline_item)
            .get_result::<TimelineItem>(&mut conn)
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

pub fn create_timeline_item_to(timeline_item_to: (TimelineItem, String)) -> bool {
    let timeline_item_to = NewTimelineItemTo::from(timeline_item_to);
    if let Ok(mut conn) = POOL.get() {
        diesel::insert_into(timeline_to::table)
            .values(&timeline_item_to)
            .get_result::<TimelineItemTo>(&mut conn)
            .is_ok()
    } else {
        false
    }
}

pub fn create_timeline_item_cc(timeline_item_cc: (TimelineItem, String)) -> bool {
    let timeline_item_cc = NewTimelineItemCc::from(timeline_item_cc);
    if let Ok(mut conn) = POOL.get() {
        diesel::insert_into(timeline_cc::table)
            .values(&timeline_item_cc)
            .get_result::<TimelineItemCc>(&mut conn)
            .is_ok()
    } else {
        false
    }
}
