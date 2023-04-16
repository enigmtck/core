use crate::activity_pub::ApLike;
use crate::db::Db;
use crate::schema::remote_likes;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[table_name = "remote_likes"]
pub struct NewRemoteLike {
    pub object_id: String,
    pub actor: String,
    pub ap_id: String,
}

impl From<ApLike> for NewRemoteLike {
    fn from(activity: ApLike) -> NewRemoteLike {
        NewRemoteLike {
            object_id: activity.object,
            actor: activity.actor.to_string(),
            ap_id: activity.id.unwrap_or_default(),
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[table_name = "remote_likes"]
pub struct RemoteLike {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ap_id: String,
    pub actor: String,
    pub object_id: String,
}

pub async fn create_remote_like(conn: &Db, remote_like: NewRemoteLike) -> Option<RemoteLike> {
    match conn
        .run(move |c| {
            diesel::insert_into(remote_likes::table)
                .values(&remote_like)
                .get_result::<RemoteLike>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => None,
    }
}

pub async fn delete_remote_like_by_actor_and_object_id(
    conn: &Db,
    actor: String,
    object_id: String,
) -> bool {
    conn.run(move |c| {
        diesel::delete(remote_likes::table)
            .filter(remote_likes::actor.eq(actor))
            .filter(remote_likes::object_id.eq(object_id))
            .execute(c)
    })
    .await
    .is_ok()
}
