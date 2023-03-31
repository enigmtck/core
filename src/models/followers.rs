use crate::activity_pub::{ApActivity, ApObject};
use crate::db::Db;
use crate::schema::followers;
use diesel::prelude::*;

use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[table_name = "followers"]
pub struct NewFollower {
    pub profile_id: i32,
    pub ap_id: String,
    pub actor: String,
    pub followed_ap_id: String,
    pub uuid: String,
}

impl From<ApActivity> for NewFollower {
    fn from(activity: ApActivity) -> NewFollower {
        let mut o = Option::<String>::None;

        if let ApObject::Plain(x) = activity.object {
            o = Some(x);
        };

        NewFollower {
            ap_id: activity.id.unwrap(),
            actor: activity.actor,
            followed_ap_id: o.unwrap_or_default(),
            uuid: Uuid::new_v4().to_string(),
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
    pub uuid: String,
}

pub async fn create_follower(conn: &Db, follower: NewFollower) -> Option<Follower> {
    if let Ok(x) = conn
        .run(move |c| {
            diesel::insert_into(followers::table)
                .values(&follower)
                .get_result::<Follower>(c)
        })
        .await
    {
        Some(x)
    } else {
        Option::None
    }
}

pub async fn get_follower_by_uuid(conn: &Db, uuid: String) -> Option<Follower> {
    if let Ok(x) = conn
        .run(move |c| {
            followers::table
                .filter(followers::uuid.eq(uuid))
                .first::<Follower>(c)
        })
        .await
    {
        Option::from(x)
    } else {
        Option::None
    }
}

pub async fn delete_follower_by_ap_id(conn: &Db, ap_id: String) -> bool {
    conn.run(move |c| {
        diesel::delete(followers::table)
            .filter(followers::ap_id.eq(ap_id))
            .execute(c)
    })
    .await
    .is_ok()
}

pub async fn get_followers_by_profile_id(conn: &Db, profile_id: i32) -> Vec<Follower> {
    if let Ok(x) = conn
        .run(move |c| {
            followers::table
                .filter(followers::profile_id.eq(profile_id))
                .order_by(followers::created_at.desc())
                .get_results::<Follower>(c)
        })
        .await
    {
        x
    } else {
        vec![]
    }
}
