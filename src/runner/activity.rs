use diesel::prelude::*;

use crate::{models::remote_activities::RemoteActivity, schema::remote_activities};

use super::POOL;

pub fn get_remote_activity_by_apid(ap_id: String) -> Option<RemoteActivity> {
    if let Ok(conn) = POOL.get() {
        match remote_activities::table
            .filter(remote_activities::ap_id.eq(ap_id))
            .first::<RemoteActivity>(&conn)
        {
            Ok(x) => Option::from(x),
            Err(_) => Option::None,
        }
    } else {
        None
    }
}
