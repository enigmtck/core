use crate::activity_pub::ApFollow;
use crate::db::Db;
use crate::schema::follows;
use crate::MaybeReference;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use super::profiles::get_profile_by_ap_id;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[table_name = "follows"]
pub struct NewFollow {
    pub ap_object: String,
    pub actor: String,
    pub uuid: String,
    pub profile_id: Option<i32>,
}

impl TryFrom<ApFollow> for NewFollow {
    type Error = &'static str;

    fn try_from(follow: ApFollow) -> Result<Self, Self::Error> {
        if let MaybeReference::Reference(object) = follow.object {
            Ok(NewFollow {
                ap_object: object,
                actor: follow.actor.to_string(),
                uuid: uuid::Uuid::new_v4().to_string(),
                profile_id: None,
            })
        } else {
            Err("INCORRECT OBJECT TYPE")
        }
    }
}

impl NewFollow {
    pub async fn link(&mut self, conn: &Db) -> Self {
        self.profile_id = {
            if let Some(profile) = get_profile_by_ap_id(conn, self.clone().actor).await {
                Some(profile.id)
            } else {
                None
            }
        };

        self.clone()
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "follows"]
pub struct Follow {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub profile_id: Option<i32>,
    pub ap_object: String,
    pub actor: String,
}

pub async fn get_follow_by_uuid(conn: &Db, uuid: String) -> Option<Follow> {
    match conn
        .run(move |c| {
            follows::table
                .filter(follows::uuid.eq(uuid))
                .first::<Follow>(c)
        })
        .await
    {
        Ok(x) => Option::from(x),
        Err(_) => Option::None,
    }
}

pub async fn get_follow_by_ap_object_and_profile(
    conn: &crate::db::Db,
    ap_object: String,
    profile_id: i32,
) -> Option<Follow> {
    match conn
        .run(move |c| {
            follows::table
                .filter(follows::ap_object.eq(ap_object))
                .filter(follows::profile_id.eq(profile_id))
                .first::<Follow>(c)
        })
        .await
    {
        Ok(x) => Option::from(x),
        Err(_) => Option::None,
    }
}

pub async fn create_follow(conn: &Db, follow: NewFollow) -> Option<Follow> {
    match conn
        .run(move |c| {
            diesel::insert_into(follows::table)
                .values(&follow)
                .get_result::<Follow>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => None,
    }
}

pub async fn delete_follow_by_actor_and_to(conn: &Db, actor: String, object: String) -> bool {
    conn.run(move |c| {
        diesel::delete(follows::table)
            .filter(follows::actor.eq(actor))
            .filter(follows::ap_object.eq(object))
            .execute(c)
    })
    .await
    .is_ok()
}
