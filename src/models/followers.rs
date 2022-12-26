use crate::activity_pub::{ApActivity, ApObject};
use crate::schema::followers;
use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[table_name = "followers"]
pub struct NewFollower {
    pub profile_id: i32,
    pub ap_id: String,
    pub actor: String,
    pub followed_ap_id: String,
}

impl From<ApActivity> for NewFollower {
    fn from(activity: ApActivity) -> NewFollower {
        let mut o = Option::<String>::None;

        if let ApObject::Plain(x) = activity.object {
            o = Some(x);
        };

        NewFollower {
            ap_id: activity.base.id.unwrap(),
            actor: activity.actor,
            followed_ap_id: o.unwrap_or_default(),
            ..Default::default()
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "followers"]
pub struct Follower {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile_id: i32,
    pub ap_id: String,
    pub actor: String,
    pub followed_ap_id: String,
}
