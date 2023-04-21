use crate::activity_pub::ApAnnounce;
use crate::db::Db;
use crate::schema::announces;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::profiles::get_profile_by_ap_id;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[table_name = "announces"]
pub struct NewAnnounce {
    pub object_ap_id: String,
    pub ap_to: Value,
    pub cc: Option<Value>,
    pub actor: String,
    pub uuid: String,
    pub profile_id: Option<i32>,
}

impl From<ApAnnounce> for NewAnnounce {
    fn from(activity: ApAnnounce) -> Self {
        NewAnnounce {
            object_ap_id: activity.object,
            ap_to: serde_json::to_value(activity.to.multiple()).unwrap(),
            cc: activity.cc.map(|cc| serde_json::to_value(cc).unwrap()),
            actor: activity.actor.to_string(),
            uuid: uuid::Uuid::new_v4().to_string(),
            profile_id: None,
        }
    }
}

impl NewAnnounce {
    pub async fn link(&mut self, conn: &Db) -> Self {
        if let Some(profile) = get_profile_by_ap_id(conn, self.clone().actor).await {
            self.profile_id = Some(profile.id);
        };

        self.clone()
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "announces"]
pub struct Announce {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile_id: Option<i32>,
    pub uuid: String,
    pub actor: String,
    pub ap_to: Value,
    pub cc: Option<Value>,
    pub object_ap_id: String,
}

pub async fn get_announce_by_uuid(conn: &Db, uuid: String) -> Option<Announce> {
    conn.run(move |c| {
        announces::table
            .filter(announces::uuid.eq(uuid))
            .first::<Announce>(c)
    })
    .await
    .ok()
}

pub async fn create_announce(conn: &Db, announce: NewAnnounce) -> Option<Announce> {
    conn.run(move |c| {
        diesel::insert_into(announces::table)
            .values(&announce)
            .get_result::<Announce>(c)
    })
    .await
    .ok()
}

pub async fn delete_announce_by_actor_and_object_ap_id(
    conn: &Db,
    actor: String,
    object_ap_id: String,
) -> bool {
    conn.run(move |c| {
        diesel::delete(announces::table)
            .filter(announces::actor.eq(actor))
            .filter(announces::object_ap_id.eq(object_ap_id))
            .execute(c)
    })
    .await
    .is_ok()
}
