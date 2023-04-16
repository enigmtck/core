use crate::activity_pub::{ApInvite, ApJoin, ApObject};
use crate::schema::remote_encrypted_sessions;
use crate::MaybeReference;
use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug)]
#[table_name = "remote_encrypted_sessions"]
pub struct NewRemoteEncryptedSession {
    pub profile_id: i32,
    pub actor: String,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: String,
    pub attributed_to: String,
    pub instrument: Value,
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
#[table_name = "remote_encrypted_sessions"]
pub struct RemoteEncryptedSession {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub profile_id: i32,
    pub actor: String,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: String,
    pub attributed_to: String,
    pub instrument: Value,
    pub reference: Option<String>,
}
