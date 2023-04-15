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
        Ok(conn) => {
            for ap_id in ap_ids {
                let ap_id = ap_id.as_str().unwrap().to_string();
                log::debug!("looking for ap_id: {}", ap_id);

                match remote_notes::table
                    .filter(remote_notes::ap_id.eq(ap_id))
                    .first::<RemoteNote>(&conn)
                {
                    Ok(remote_note) => {
                        if remote_note.kind == "Note" {
                            update_timeline_items(remote_note.clone().into());
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

pub fn add_to_timeline(ap_to: Option<Value>, cc: Option<Value>, timeline_item: TimelineItem) {
    if let Some(ap_to) = ap_to {
        if let Ok(to_vec) = serde_json::from_value::<Vec<String>>(ap_to.clone()) {
            //let to_vec: Vec<String> = serde_json::from_value(ap_to).unwrap();

            for to in to_vec {
                create_timeline_item_to((timeline_item.clone(), to.clone()).into());

                if get_leader_by_endpoint(to.clone()).is_some() {
                    for follower in get_follower_profiles_by_endpoint(to) {
                        if let Some(follower) = follower.2 {
                            log::debug!("adding to for {}", follower.username);

                            let follower =
                                format!("{}/user/{}", &*crate::SERVER_URL, follower.username);
                            create_timeline_item_to((timeline_item.clone(), follower).into());
                        }
                    }
                }
            }
        } else {
            log::error!("TO VALUE NOT A VEC: {ap_to:#?}");
        }
    }

    if let Some(cc) = cc {
        if let Ok(cc_vec) = serde_json::from_value::<Vec<String>>(cc.clone()) {
            //if let Ok(cc_vec) = serde_json::from_value::<Vec<String>>(cc) {
            for cc in cc_vec {
                create_timeline_item_cc((timeline_item.clone(), cc.clone()).into());

                if get_leader_by_endpoint(cc.clone()).is_some() {
                    for follower in get_follower_profiles_by_endpoint(cc) {
                        if let Some(follower) = follower.2 {
                            log::debug!("adding cc for {}", follower.username);

                            let follower =
                                format!("{}/user/{}", &*crate::SERVER_URL, follower.username);
                            create_timeline_item_cc((timeline_item.clone(), follower).into());
                        }
                    }
                }
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
        Ok(conn) => {
            match diesel::delete(timeline::table.filter(timeline::ap_id.eq(ap_id))).execute(&conn) {
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
    if let Ok(conn) = POOL.get() {
        match timeline::table
            .filter(timeline::ap_id.eq(ap_id))
            .first::<TimelineItem>(&conn)
        {
            Ok(x) => Option::from(x),
            Err(_) => Option::None,
        }
    } else {
        Option::None
    }
}

pub fn update_timeline_items(timeline_item: NewTimelineItem) -> Vec<TimelineItem> {
    if let Ok(conn) = POOL.get() {
        match diesel::update(timeline::table.filter(timeline::ap_id.eq(timeline_item.ap_id)))
            .set(timeline::content.eq(timeline_item.content))
            .get_results::<TimelineItem>(&conn)
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
    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(timeline::table)
            .values(&timeline_item)
            .get_result::<TimelineItem>(&conn)
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

pub fn create_timeline_item_to(timeline_item_to: NewTimelineItemTo) -> Option<TimelineItemTo> {
    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(timeline_to::table)
            .values(&timeline_item_to)
            .get_result::<TimelineItemTo>(&conn)
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

pub fn create_timeline_item_cc(timeline_item_cc: NewTimelineItemCc) -> Option<TimelineItemCc> {
    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(timeline_cc::table)
            .values(&timeline_item_cc)
            .get_result::<TimelineItemCc>(&conn)
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
