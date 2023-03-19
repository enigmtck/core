use crate::activity_pub::ApActivity;
use crate::db::Db;
use crate::schema::remote_activities;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[table_name = "remote_activities"]
pub struct NewRemoteActivity {
    pub context: Option<Value>,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub actor: String,
    pub published: Option<String>,
    pub ap_object: Option<Value>,
}

impl From<ApActivity> for NewRemoteActivity {
    fn from(activity: ApActivity) -> NewRemoteActivity {
        NewRemoteActivity {
            context: Option::from(serde_json::to_value(&activity.context).unwrap()),
            kind: activity.kind.to_string(),
            ap_id: activity.id.unwrap_or_default(),
            ap_to: activity.to.map(|to| serde_json::to_value(to).unwrap()),
            cc: Option::from(serde_json::to_value(activity.cc.unwrap_or_default()).unwrap()),
            actor: activity.actor,
            published: activity.published,
            ap_object: Option::from(serde_json::to_value(&activity.object).unwrap()),
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "remote_activities"]
pub struct RemoteActivity {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub context: Option<Value>,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub actor: String,
    pub published: Option<String>,
    pub ap_object: Option<Value>,
}

pub async fn create_remote_activity(
    conn: &Db,
    remote_activity: NewRemoteActivity,
) -> Option<RemoteActivity> {
    match conn
        .run(move |c| {
            diesel::insert_into(remote_activities::table)
                .values(&remote_activity)
                .on_conflict(remote_activities::ap_id)
                .do_nothing()
                .get_result::<RemoteActivity>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(e) => {
            log::debug!("failed to create remote_activity (probably a duplicate): {e:#?}");
            Option::None
        }
    }
}

pub async fn get_remote_activity_by_ap_id(conn: &Db, ap_id: String) -> Option<RemoteActivity> {
    match conn
        .run(move |c| {
            remote_activities::table
                .filter(remote_activities::ap_id.eq(ap_id))
                .first::<RemoteActivity>(c)
        })
        .await
    {
        Ok(x) => Option::from(x),
        Err(_) => Option::None,
    }
}
