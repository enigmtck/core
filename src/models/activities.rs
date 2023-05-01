use crate::db::Db;
use crate::schema::{activities, notes, profiles, remote_notes};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::notes::Note;
use super::profiles::Profile;
use super::remote_notes::RemoteNote;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[diesel(table_name = activities)]
pub struct NewActivity {
    pub kind: String,
    pub uuid: String,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub profile_id: Option<i32>,
    pub target_note_id: Option<i32>,
    pub target_remote_note_id: Option<i32>,
    pub target_profile_id: Option<i32>,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = activities)]
pub struct Activity {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile_id: i32,
    pub kind: String,
    pub uuid: String,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub target_note_id: Option<i32>,
    pub target_remote_note_id: Option<i32>,
    pub target_profile_id: Option<i32>,
}

pub async fn create_activity(conn: &Db, activity: NewActivity) -> Option<Activity> {
    conn.run(move |c| {
        diesel::insert_into(activities::table)
            .values(&activity)
            .get_result::<Activity>(c)
    })
    .await
    .ok()
}

pub type ExtendedActivity = (Activity, Option<Note>, Option<RemoteNote>, Option<Profile>);
pub async fn get_activity_by_uuid(conn: &Db, uuid: String) -> Option<ExtendedActivity> {
    conn.run(move |c| {
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
            .first::<ExtendedActivity>(c)
    })
    .await
    .ok()
}
