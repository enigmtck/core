use crate::activity_pub::{ApAccept, ApActivity};
use crate::db::Db;
use crate::helper::{get_local_identifier, LocalIdentifierType};
use crate::schema::{leaders, remote_actors};
use crate::{MaybeReference, POOL};
use diesel::prelude::*;
use diesel::Insertable;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::profiles::Profile;
use super::remote_actors::RemoteActor;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::leaders::Leader;
        pub use crate::models::pg::leaders::create_leader;
        pub use crate::models::pg::leaders::update_leader_by_uuid;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::leaders::Leader;
        pub use crate::models::sqlite::leaders::create_leader;
        pub use crate::models::sqlite::leaders::update_leader_by_uuid;
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[diesel(table_name = leaders)]
pub struct NewLeader {
    pub profile_id: i32,
    pub actor: String,
    pub leader_ap_id: String,
    pub uuid: String,
    pub accept_ap_id: Option<String>,
    pub accepted: Option<bool>,
    pub follow_ap_id: Option<String>,
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
                follow_ap_id: follow.id,
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

pub async fn delete_leader(conn: &Db, leader_id: i32) -> bool {
    conn.run(move |c| diesel::delete(leaders::table.find(leader_id)).execute(c))
        .await
        .is_ok()
}

pub async fn delete_leader_by_ap_id_and_profile_id(
    conn: Option<&Db>,
    leader_ap_id: String,
    profile_id: i32,
) -> bool {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::delete(
                    leaders::table
                        .filter(leaders::profile_id.eq(profile_id))
                        .filter(leaders::leader_ap_id.eq(leader_ap_id)),
                )
                .execute(c)
            })
            .await
            .is_ok(),
        None => POOL.get().map_or(false, |mut pool| {
            diesel::delete(
                leaders::table
                    .filter(leaders::profile_id.eq(profile_id))
                    .filter(leaders::leader_ap_id.eq(leader_ap_id)),
            )
            .execute(&mut pool)
            .is_ok()
        }),
    }
}

pub async fn get_leader_by_profile_id_and_ap_id(
    conn: Option<&Db>,
    profile_id: i32,
    leader_ap_id: String,
) -> Option<Leader> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                leaders::table
                    .filter(
                        leaders::profile_id
                            .eq(profile_id)
                            .and(leaders::leader_ap_id.eq(leader_ap_id)),
                    )
                    .first::<Leader>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            leaders::table
                .filter(
                    leaders::profile_id
                        .eq(profile_id)
                        .and(leaders::leader_ap_id.eq(leader_ap_id)),
                )
                .first::<Leader>(&mut pool)
                .ok()
        }
    }
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
