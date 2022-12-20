use crate::activity_pub::Actor;
use crate::schema::remote_actors;
use chrono::{DateTime, Utc};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable)]
#[table_name = "remote_actors"]
pub struct NewRemoteActor {
    pub context: Value,
    pub kind: String,
    pub ap_id: String,
    pub name: String,
    pub preferred_username: String,
    pub summary: String,
    pub inbox: String,
    pub outbox: String,
    pub followers: String,
    pub following: String,
    pub public_key: Value,
}

impl From<Actor> for NewRemoteActor {
    fn from(actor: Actor) -> NewRemoteActor {
        NewRemoteActor {
            context: serde_json::to_value(&actor.context).unwrap(),
            kind: actor.kind,
            ap_id: actor.id,
            name: actor.name,
            preferred_username: actor.preferred_username,
            summary: actor.summary,
            inbox: actor.inbox,
            outbox: actor.outbox,
            followers: actor.followers,
            following: actor.following,
            public_key: serde_json::to_value(&actor.public_key).unwrap(),
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone)]
#[table_name = "remote_actors"]
pub struct RemoteActor {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub context: Value,
    pub kind: String,
    pub ap_id: String,
    pub name: String,
    pub preferred_username: Option<String>,
    pub summary: Option<String>,
    pub inbox: String,
    pub outbox: String,
    pub followers: String,
    pub following: String,
    pub liked: Option<String>,
    pub public_key: Option<Value>,
}
