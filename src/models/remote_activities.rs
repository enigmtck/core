use crate::activity_pub::ApActivity;
use crate::db::Db;
use crate::schema::remote_activities;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[diesel(table_name = remote_activities)]
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

// whew this sucks. a vestige of trying to consolidate all Activity in to
// one table. I can do better than this.
impl From<ApActivity> for NewRemoteActivity {
    fn from(activity: ApActivity) -> NewRemoteActivity {
        match activity {
            ApActivity::Delete(activity) => NewRemoteActivity {
                context: Some(serde_json::to_value(&activity.context).unwrap()),
                kind: activity.kind.to_string(),
                ap_id: activity.id.unwrap_or_default(),
                ap_to: Some(serde_json::to_value(activity.to).unwrap()),
                cc: None,
                actor: activity.actor.to_string(),
                published: None,
                ap_object: Option::from(serde_json::to_value(&activity.object).unwrap()),
            },
            ApActivity::Follow(activity) => NewRemoteActivity {
                context: Some(serde_json::to_value(&activity.context).unwrap()),
                kind: activity.kind.to_string(),
                ap_id: activity.id.unwrap_or_default(),
                ap_to: None,
                cc: None,
                actor: activity.actor.to_string(),
                published: None,
                ap_object: Option::from(serde_json::to_value(&activity.object).unwrap()),
            },
            ApActivity::Accept(activity) => NewRemoteActivity {
                context: Some(serde_json::to_value(&activity.context).unwrap()),
                kind: activity.kind.to_string(),
                ap_id: activity.id.unwrap_or_default(),
                ap_to: None,
                cc: None,
                actor: activity.actor.to_string(),
                published: None,
                ap_object: Option::from(serde_json::to_value(&activity.object).unwrap()),
            },
            ApActivity::Like(activity) => NewRemoteActivity {
                context: Some(serde_json::to_value(&activity.context).unwrap()),
                kind: activity.kind.to_string(),
                ap_id: activity.id.unwrap_or_default(),
                ap_to: activity.to.map(|to| serde_json::to_value(to).unwrap()),
                cc: None,
                actor: activity.actor.to_string(),
                published: None,
                ap_object: Some(serde_json::to_value(&activity.object).unwrap()),
            },
            ApActivity::Announce(activity) => NewRemoteActivity {
                context: Some(serde_json::to_value(&activity.context).unwrap()),
                kind: activity.kind.to_string(),
                ap_id: activity.id.unwrap_or_default(),
                ap_to: Some(serde_json::to_value(activity.to).unwrap()),
                cc: activity.cc.map(|cc| serde_json::to_value(cc).unwrap()),
                actor: activity.actor.to_string(),
                published: None,
                ap_object: Option::from(serde_json::to_value(&activity.object).unwrap()),
            },
            ApActivity::Create(activity) => NewRemoteActivity {
                context: Some(serde_json::to_value(&activity.context).unwrap()),
                kind: activity.kind.to_string(),
                ap_id: activity.id.unwrap_or_default(),
                ap_to: Some(serde_json::to_value(activity.to).unwrap()),
                cc: activity.cc.map(|cc| serde_json::to_value(cc).unwrap()),
                actor: activity.actor.to_string(),
                published: activity.published,
                ap_object: Some(serde_json::to_value(&activity.object).unwrap()),
            },
            ApActivity::Invite(activity) => NewRemoteActivity {
                context: Some(serde_json::to_value(&activity.context).unwrap()),
                kind: activity.kind.to_string(),
                ap_id: activity.id.unwrap_or_default(),
                ap_to: Some(serde_json::to_value(activity.to).unwrap()),
                cc: None,
                actor: activity.actor.to_string(),
                published: None,
                ap_object: Option::from(serde_json::to_value(&activity.object).unwrap()),
            },
            ApActivity::Join(activity) => NewRemoteActivity {
                context: Some(serde_json::to_value(&activity.context).unwrap()),
                kind: activity.kind.to_string(),
                ap_id: activity.id.unwrap_or_default(),
                ap_to: Some(serde_json::to_value(activity.to).unwrap()),
                cc: None,
                actor: activity.actor.to_string(),
                published: None,
                ap_object: Option::from(serde_json::to_value(&activity.object).unwrap()),
            },
            ApActivity::Undo(activity) => NewRemoteActivity {
                context: Some(serde_json::to_value(&activity.context).unwrap()),
                kind: activity.kind.to_string(),
                ap_id: activity.id.unwrap_or_default(),
                ap_to: None,
                cc: None,
                actor: activity.actor.to_string(),
                published: None,
                ap_object: Some(serde_json::to_value(&activity.object).unwrap()),
            },
            ApActivity::Update(activity) => NewRemoteActivity {
                context: Some(serde_json::to_value(&activity.context).unwrap()),
                kind: activity.kind.to_string(),
                ap_id: activity.id.unwrap_or_default(),
                ap_to: Some(serde_json::to_value(activity.to).unwrap()),
                cc: None,
                actor: activity.actor.to_string(),
                published: None,
                ap_object: Some(serde_json::to_value(&activity.object).unwrap()),
            },
            ApActivity::Block(activity) => NewRemoteActivity {
                context: Some(serde_json::to_value(&activity.context).unwrap()),
                kind: activity.kind.to_string(),
                ap_id: activity.id.unwrap_or_default(),
                ap_to: None,
                cc: None,
                actor: activity.actor.to_string(),
                published: None,
                ap_object: Some(serde_json::to_value(&activity.object).unwrap()),
            },
            ApActivity::Add(activity) => NewRemoteActivity {
                context: Some(serde_json::to_value(&activity.context).unwrap()),
                kind: activity.kind.to_string(),
                ap_id: uuid::Uuid::new_v4().to_string(),
                ap_to: None,
                cc: None,
                actor: activity.actor.to_string(),
                published: None,
                ap_object: Some(serde_json::to_value(&activity.object).unwrap()),
            },
            ApActivity::Remove(activity) => NewRemoteActivity {
                context: Some(serde_json::to_value(&activity.context).unwrap()),
                kind: activity.kind.to_string(),
                ap_id: uuid::Uuid::new_v4().to_string(),
                ap_to: None,
                cc: None,
                actor: activity.actor.to_string(),
                published: None,
                ap_object: Some(serde_json::to_value(&activity.object).unwrap()),
            },
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = remote_activities)]
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
    conn.run(move |c| {
        diesel::insert_into(remote_activities::table)
            .values(&remote_activity)
            .on_conflict(remote_activities::ap_id)
            .do_nothing()
            .get_result::<RemoteActivity>(c)
    })
    .await
    .ok()
}

pub async fn get_remote_activity_by_ap_id(conn: &Db, ap_id: String) -> Option<RemoteActivity> {
    conn.run(move |c| {
        remote_activities::table
            .filter(remote_activities::ap_id.eq(ap_id))
            .first::<RemoteActivity>(c)
    })
    .await
    .ok()
}
