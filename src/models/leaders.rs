use crate::activity_pub::{ApAccept, ApActivity};
use crate::db::Db;
use crate::helper::{get_local_identifier, LocalIdentifierType};
use crate::schema::{leaders, remote_actors};
use crate::MaybeReference;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::profiles::Profile;
use super::remote_actors::RemoteActor;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[diesel(table_name = leaders)]
pub struct NewLeader {
    pub profile_id: i32,
    pub actor: String,
    pub leader_ap_id: String,
    pub uuid: String,
    pub accept_ap_id: Option<String>,
    pub accepted: Option<bool>,
}

impl TryFrom<ApAccept> for NewLeader {
    type Error = &'static str;

    fn try_from(accept: ApAccept) -> Result<Self, Self::Error> {
        if let MaybeReference::Actual(ApActivity::Follow(follow)) = accept.object {
            Ok(NewLeader {
                actor: follow.actor.to_string(),
                leader_ap_id: accept.actor.to_string(),
                uuid: Uuid::new_v4().to_string(),
                accept_ap_id: accept.id,
                accepted: Some(true),
                profile_id: -1,
            })
        } else {
            Err("ACCEPT DOES NOT CONTAIN A VALID FOLLOW OBJECT")
        }
    }
}

impl NewLeader {
    pub fn link(&mut self, profile: Profile) -> &mut Self {
        if let Some(id) = get_local_identifier(self.actor.clone()) {
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
#[diesel(table_name = leaders)]
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

pub async fn get_leader_by_actor_ap_id_and_profile(
    conn: &crate::db::Db,
    ap_id: String,
    profile_id: i32,
) -> Option<Leader> {
    use crate::schema::leaders::dsl::{leader_ap_id, leaders, profile_id as pid};

    conn.run(move |c| {
        leaders
            .filter(leader_ap_id.eq(ap_id))
            .filter(pid.eq(profile_id))
            .first::<Leader>(c)
    })
    .await
    .ok()
}

pub async fn create_leader(conn: &Db, leader: NewLeader) -> Option<Leader> {
    conn.run(move |c| {
        diesel::insert_into(leaders::table)
            .values(&leader)
            .get_result::<Leader>(c)
    })
    .await
    .ok()
}

pub async fn delete_leader(conn: &Db, leader_id: i32) -> bool {
    conn.run(move |c| diesel::delete(leaders::table.find(leader_id)).execute(c))
        .await
        .is_ok()
}

pub async fn delete_leader_by_ap_id_and_profile(
    conn: &Db,
    leader_ap_id: String,
    profile_id: i32,
) -> bool {
    conn.run(move |c| {
        diesel::delete(
            leaders::table
                .filter(leaders::profile_id.eq(profile_id))
                .filter(leaders::leader_ap_id.eq(leader_ap_id)),
        )
        .execute(c)
    })
    .await
    .is_ok()
}

pub async fn get_leader_by_profile_id_and_ap_id(
    conn: &Db,
    profile_id: i32,
    leader_ap_id: String,
) -> Option<Leader> {
    conn.run(move |c| {
        leaders::table
            .filter(
                leaders::profile_id
                    .eq(profile_id)
                    .and(leaders::leader_ap_id.eq(leader_ap_id)),
            )
            .first::<Leader>(c)
    })
    .await
    .ok()
}

pub async fn update_leader_by_uuid(
    conn: &Db,
    leader_uuid: String,
    accept_ap_id: String,
) -> Option<Leader> {
    conn.run(move |c| {
        diesel::update(leaders::table.filter(leaders::uuid.eq(leader_uuid)))
            .set((
                leaders::accept_ap_id.eq(accept_ap_id),
                leaders::accepted.eq(true),
            ))
            .get_result::<Leader>(c)
    })
    .await
    .ok()
}

pub async fn get_leaders_by_profile_id(
    conn: &Db,
    profile_id: i32,
) -> Vec<(Leader, Option<RemoteActor>)> {
    conn.run(move |c| {
        leaders::table
            .filter(leaders::profile_id.eq(profile_id))
            .left_join(remote_actors::table.on(leaders::leader_ap_id.eq(remote_actors::ap_id)))
            .order_by(leaders::created_at.desc())
            .get_results::<(Leader, Option<RemoteActor>)>(c)
    })
    .await
    .unwrap_or(vec![])
}
