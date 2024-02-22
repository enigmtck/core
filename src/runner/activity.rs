use diesel::prelude::*;

use crate::{
    models::activities::{Activity, ExtendedActivity},
    schema::{activities, notes, profiles, remote_actors, remote_notes},
    POOL,
};

//pub type ExtendedActivity = (Activity, Option<Note>, Option<RemoteNote>, Option<Profile>, Option<RemoteActor>);
pub fn get_activity_by_uuid(uuid: String) -> Option<ExtendedActivity> {
    if let Ok(mut conn) = POOL.get() {
        activities::table
            .filter(activities::uuid.eq(uuid))
            .left_join(notes::table.on(activities::target_note_id.eq(notes::id.nullable())))
            .left_join(
                remote_notes::table
                    .on(activities::target_remote_note_id.eq(remote_notes::id.nullable())),
            )
            .left_join(
                profiles::table.on(activities::target_profile_id.eq(profiles::id.nullable())),
            )
            .left_join(
                remote_actors::table
                    .on(activities::target_remote_actor_id.eq(remote_actors::id.nullable())),
            )
            .first::<ExtendedActivity>(&mut conn)
            .ok()
    } else {
        None
    }
}

pub fn get_activity(id: i32) -> Option<ExtendedActivity> {
    if let Ok(mut conn) = POOL.get() {
        activities::table
            .find(id)
            .left_join(notes::table.on(activities::target_note_id.eq(notes::id.nullable())))
            .left_join(
                remote_notes::table
                    .on(activities::target_remote_note_id.eq(remote_notes::id.nullable())),
            )
            .left_join(
                profiles::table.on(activities::target_profile_id.eq(profiles::id.nullable())),
            )
            .left_join(
                remote_actors::table
                    .on(activities::target_remote_actor_id.eq(remote_actors::id.nullable())),
            )
            .first::<ExtendedActivity>(&mut conn)
            .ok()
    } else {
        None
    }
}

pub fn revoke_activity_by_uuid(uuid: String) -> Option<Activity> {
    if let Ok(mut conn) = POOL.get() {
        diesel::update(activities::table.filter(activities::uuid.eq(uuid)))
            .set(activities::revoked.eq(true))
            .get_result::<Activity>(&mut conn)
            .ok()
    } else {
        None
    }
}

pub fn revoke_activity_by_apid(ap_id: &str) -> Option<Activity> {
    if let Ok(mut conn) = POOL.get() {
        diesel::update(activities::table.filter(activities::ap_id.eq(ap_id)))
            .set(activities::revoked.eq(true))
            .get_result::<Activity>(&mut conn)
            .ok()
    } else {
        None
    }
}
