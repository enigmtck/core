use crate::activity_pub::ApLike;
use crate::db::Db;
use crate::schema::likes;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use super::profiles::get_profile_by_ap_id;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = likes)]
pub struct NewLike {
    pub object_ap_id: String,
    pub ap_to: String,
    pub actor: String,
    pub uuid: String,
    pub profile_id: Option<i32>,
}

impl From<ApLike> for NewLike {
    fn from(like: ApLike) -> Self {
        NewLike {
            object_ap_id: like.object.to_string(),
            // GROSS
            ap_to: like.to.unwrap_or_default().single().unwrap().to_string(),
            actor: like.actor.to_string(),
            uuid: uuid::Uuid::new_v4().to_string(),
            profile_id: None,
        }
    }
}

impl NewLike {
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
#[diesel(table_name = likes)]
pub struct Like {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub profile_id: Option<i32>,
    pub ap_to: String,
    pub actor: String,
    pub object_ap_id: String,
}

pub async fn get_like_by_uuid(conn: &Db, uuid: String) -> Option<Like> {
    conn.run(move |c| likes::table.filter(likes::uuid.eq(uuid)).first::<Like>(c))
        .await
        .ok()
}

pub async fn create_like(conn: &Db, like: NewLike) -> Option<Like> {
    conn.run(move |c| {
        diesel::insert_into(likes::table)
            .values(&like)
            .get_result::<Like>(c)
    })
    .await
    .ok()
}

pub async fn delete_like_by_actor_and_object_ap_id(
    conn: &Db,
    actor: String,
    object_ap_id: String,
) -> bool {
    conn.run(move |c| {
        diesel::delete(likes::table)
            .filter(likes::actor.eq(actor))
            .filter(likes::object_ap_id.eq(object_ap_id))
            .execute(c)
    })
    .await
    .is_ok()
}
