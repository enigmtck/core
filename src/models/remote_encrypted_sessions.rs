use crate::activity_pub::{ApInvite, ApJoin, ApObject};
use crate::schema::remote_encrypted_sessions;
use crate::{db::Db, MaybeReference, POOL};
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[diesel(table_name = remote_encrypted_sessions)]
pub struct NewRemoteEncryptedSession {
    pub profile_id: i32,
    pub actor: String,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: String,
    pub attributed_to: String,
    pub instrument: String,
    pub reference: Option<String>,
}

type IdentifiedApInvite = (ApInvite, i32);
impl From<IdentifiedApInvite> for NewRemoteEncryptedSession {
    fn from((activity, profile_id): IdentifiedApInvite) -> NewRemoteEncryptedSession {
        if let MaybeReference::Actual(ApObject::Session(session)) = activity.object {
            NewRemoteEncryptedSession {
                actor: activity.actor.to_string(),
                kind: activity.kind.to_string(),
                profile_id,
                ap_id: session.id.unwrap(),
                ap_to: session.to.to_string(),
                attributed_to: session.attributed_to.to_string(),
                reference: session.reference,
                instrument: serde_json::to_value(session.instrument).unwrap(),
            }
        } else {
            NewRemoteEncryptedSession::default()
        }
    }
}

type IdentifiedApJoin = (ApJoin, i32);
impl From<IdentifiedApJoin> for NewRemoteEncryptedSession {
    fn from((activity, profile_id): IdentifiedApJoin) -> NewRemoteEncryptedSession {
        if let MaybeReference::Actual(ApObject::Session(session)) = activity.object {
            NewRemoteEncryptedSession {
                actor: activity.actor.to_string(),
                kind: activity.kind.to_string(),
                profile_id,
                ap_id: session.id.unwrap(),
                ap_to: session.to.to_string(),
                attributed_to: session.attributed_to.to_string(),
                reference: session.reference,
                instrument: serde_json::to_value(session.instrument).unwrap(),
            }
        } else {
            NewRemoteEncryptedSession::default()
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Deserialize, Clone, Default, Debug)]
#[diesel(table_name = remote_encrypted_sessions)]
pub struct RemoteEncryptedSession {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub profile_id: i32,
    pub actor: String,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: String,
    pub attributed_to: String,
    pub instrument: String,
    pub reference: Option<String>,
}

pub async fn get_remote_encrypted_session_by_ap_id(
    conn: Option<&Db>,
    apid: String,
) -> Option<RemoteEncryptedSession> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                remote_encrypted_sessions::table
                    .filter(remote_encrypted_sessions::ap_id.eq(apid))
                    .first::<RemoteEncryptedSession>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            remote_encrypted_sessions::table
                .filter(remote_encrypted_sessions::ap_id.eq(apid))
                .first::<RemoteEncryptedSession>(&mut pool)
                .ok()
        }
    }
}

pub async fn create_remote_encrypted_session(
    conn: &Db,
    remote_encrypted_session: NewRemoteEncryptedSession,
) -> Option<RemoteEncryptedSession> {
    if let Ok(x) = conn
        .run(move |c| {
            diesel::insert_into(remote_encrypted_sessions::table)
                .values(&remote_encrypted_session)
                .get_result::<RemoteEncryptedSession>(c)
        })
        .await
    {
        Some(x)
    } else {
        Option::None
    }
}
