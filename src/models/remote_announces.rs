use crate::activity_pub::{ApAnnounce, ApObject};
use crate::db::Db;
use crate::schema::remote_announces;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::timeline::get_timeline_item_by_ap_id;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = remote_announces)]
pub struct NewRemoteAnnounce {
    pub context: Option<String>,
    pub kind: String,
    pub ap_id: String,
    pub actor: String,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub published: String,
    pub ap_object: Value,
    pub timeline_id: Option<i32>,
}

impl NewRemoteAnnounce {
    pub async fn link(&mut self, conn: &Db) -> Self {
        self.timeline_id = {
            if let Ok(ApObject::Plain(id)) = serde_json::from_value(self.ap_object.clone()) {
                if let Some(timeline) = get_timeline_item_by_ap_id(conn, id).await {
                    Some(timeline.id)
                } else {
                    None
                }
            } else {
                None
            }
        };

        self.clone()
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Default, Debug)]
#[diesel(table_name = remote_announces)]
pub struct RemoteAnnounce {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub context: Option<String>,
    pub kind: String,
    pub ap_id: String,
    pub actor: String,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub published: String,
    pub ap_object: Value,
    pub timeline_id: Option<i32>,
    pub revoked: bool,
}

impl From<ApAnnounce> for NewRemoteAnnounce {
    fn from(activity: ApAnnounce) -> NewRemoteAnnounce {
        NewRemoteAnnounce {
            context: activity
                .context
                .map(|ctx| serde_json::to_string(&ctx).unwrap()),
            kind: activity.kind.to_string(),
            ap_id: activity.id.unwrap(),
            actor: activity.actor.to_string(),
            ap_to: Some(serde_json::to_value(activity.to).unwrap()),
            cc: activity.cc.map(|cc| serde_json::to_value(cc).unwrap()),
            published: activity.published.unwrap_or_default(),
            ap_object: serde_json::to_value(&activity.object).unwrap(),
            timeline_id: None,
        }
    }
}

pub async fn create_remote_announce(
    conn: &Db,
    remote_announce: NewRemoteAnnounce,
) -> Option<RemoteAnnounce> {
    conn.run(move |c| {
        diesel::insert_into(remote_announces::table)
            .values(&remote_announce)
            .get_result::<RemoteAnnounce>(c)
    })
    .await
    .ok()
}

pub async fn get_remote_announce_by_ap_id(
    conn: &crate::db::Db,
    ap_id: String,
) -> Option<RemoteAnnounce> {
    conn.run(move |c| {
        remote_announces::table
            .filter(remote_announces::ap_id.eq(ap_id))
            .first::<RemoteAnnounce>(c)
    })
    .await
    .ok()
}

pub async fn update_revoked_by_ap_id(conn: &Db, ap_id: String) -> Option<RemoteAnnounce> {
    conn.run(move |c| {
        diesel::update(remote_announces::table.filter(remote_announces::ap_id.eq(ap_id)))
            .set(remote_announces::revoked.eq(true))
            .get_result::<RemoteAnnounce>(c)
    })
    .await
    .ok()
}
