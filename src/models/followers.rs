use crate::activity_pub::{ApActivity, ApFollow};
use crate::db::Db;
use crate::helper::{get_local_identifier, LocalIdentifierType};
use crate::schema::followers;
use crate::MaybeReference;
use diesel::prelude::*;

use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::profiles::Profile;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[table_name = "followers"]
pub struct NewFollower {
    pub profile_id: i32,
    pub ap_id: String,
    pub actor: String,
    pub followed_ap_id: String,
    pub uuid: String,
}

// impl From<ApActivity> for NewFollower {
//     fn from(activity: ApActivity) -> NewFollower {
//         let mut o = Option::<String>::None;

//         if let MaybeReference::Reference(x) = activity.object {
//             o = Some(x);
//         };

//         NewFollower {
//             ap_id: activity.id.unwrap(),
//             actor: activity.actor,
//             followed_ap_id: o.unwrap_or_default(),
//             uuid: Uuid::new_v4().to_string(),
//             ..Default::default()
//         }
//     }
// }

impl TryFrom<ApFollow> for NewFollower {
    type Error = &'static str;

    fn try_from(follow: ApFollow) -> Result<Self, Self::Error> {
        let followed = {
            match follow.object {
                MaybeReference::Reference(followed) => Some(followed),
                _ => None,
            }
        };

        if let Some(followed) = followed {
            Ok(NewFollower {
                ap_id: follow.id.unwrap(),
                actor: follow.actor,
                followed_ap_id: followed,
                uuid: Uuid::new_v4().to_string(),
                ..Default::default()
            })
        } else {
            Err("COULD NOT BUILD NEW FOLLOWER")
        }
    }
}

impl NewFollower {
    pub fn link(&mut self, profile: Profile) -> &mut Self {
        if let Some(id) = get_local_identifier(self.followed_ap_id.clone()) {
            if id.kind == LocalIdentifierType::User
                && id.identifier.to_lowercase() == profile.username.to_lowercase()
            {
                self.profile_id = profile.id;
                self
            } else {
                self
            }
        } else {
            self
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
