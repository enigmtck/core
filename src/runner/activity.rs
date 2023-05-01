use diesel::prelude::*;

use crate::{
    models::remote_activities::{NewRemoteActivity, RemoteActivity},
    schema::remote_activities,
};

use super::POOL;

pub fn get_remote_activity_by_apid(ap_id: String) -> Option<RemoteActivity> {
    if let Ok(mut conn) = POOL.get() {
        match remote_activities::table
            .filter(remote_activities::ap_id.eq(ap_id))
            .first::<RemoteActivity>(&mut conn)
        {
            Ok(x) => Option::from(x),
            Err(_) => Option::None,
        }
    } else {
        None
    }
}

pub fn create_remote_activity(remote_activity: NewRemoteActivity) -> Option<RemoteActivity> {
    if let Ok(mut conn) = POOL.get() {
        match diesel::insert_into(remote_activities::table)
            .values(&remote_activity)
            .get_result::<RemoteActivity>(&mut conn)
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
