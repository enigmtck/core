use crate::activity_pub::{ApActivity, ApObject};
use crate::schema::leaders;
use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[table_name = "leaders"]
pub struct NewLeader {
    pub profile_id: i32,
    pub actor: String,
    pub leader_ap_id: String,
    pub uuid: String,
}

impl From<ApActivity> for NewLeader {
    fn from(activity: ApActivity) -> NewLeader {
        let mut object = Option::<String>::None;

        if let ApObject::Plain(x) = activity.object {
            object = Some(x);
        };

        NewLeader {
            actor: activity.actor,
            leader_ap_id: object.unwrap_or_default(),
            uuid: Uuid::new_v4().to_string(),
            ..Default::default()
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "leaders"]
pub struct Leader {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile_id: i32,
    pub actor: String,
    pub leader_ap_id: String,
    pub uuid: String,
    pub accept_ap_id: Option<String>,
    pub accepted: Option<bool>,
}
