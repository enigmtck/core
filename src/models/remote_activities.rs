use crate::activity_pub::ApActivity;
use crate::schema::remote_activities;
use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[table_name = "remote_activities"]
pub struct NewRemoteActivity {
    pub profile_id: i32,
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
            ap_id: activity.id.unwrap(),
            ap_to: Option::from(serde_json::to_value(activity.to.unwrap_or_default()).unwrap()),
            cc: Option::from(serde_json::to_value(activity.cc.unwrap_or_default()).unwrap()),
            actor: activity.actor,
            published: activity.published,
            ap_object: Option::from(serde_json::to_value(&activity.object).unwrap()),
            ..Default::default()
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
    pub profile_id: i32,
    pub context: Option<Value>,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: Option<Value>,
    pub cc: Option<Value>,
    pub actor: String,
    pub published: Option<String>,
    pub ap_object: Option<Value>,
}
