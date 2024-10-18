use crate::activity_pub::{ApInvite, ApJoin, ApObject};
use crate::models::to_serde;
use crate::schema::remote_encrypted_sessions;
use crate::{db::Db, MaybeReference, POOL};
use diesel::prelude::*;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::remote_encrypted_sessions::RemoteEncryptedSession;
        pub use crate::models::pg::remote_encrypted_sessions::NewRemoteEncryptedSession;
        pub use crate::models::pg::remote_encrypted_sessions::create_remote_encrypted_session;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::remote_encrypted_sessions::RemoteEncryptedSession;
        pub use crate::models::sqlite::remote_encrypted_sessions::NewRemoteEncryptedSession;
        pub use crate::models::sqlite::remote_encrypted_sessions::create_remote_encrypted_session;
    }
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
                instrument: to_serde(&Some(session.instrument)).unwrap(),
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
                instrument: to_serde(&Some(session.instrument)).unwrap(),
            }
        } else {
            NewRemoteEncryptedSession::default()
        }
    }
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
