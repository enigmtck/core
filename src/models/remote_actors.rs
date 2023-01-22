use crate::schema::remote_actors;
use crate::{activity_pub::ApActor, helper::handle_option};
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
    pub liked: Option<String>,
    pub public_key: Value,
    pub featured: Option<String>,
    pub featured_tags: Option<String>,
    pub url: Option<String>,
    pub manually_approves_followers: Option<bool>,
    pub published: Option<String>,
    pub tag: Option<Value>,
    pub attachment: Option<Value>,
    pub endpoints: Option<Value>,
    pub icon: Option<Value>,
    pub image: Option<Value>,
    pub also_known_as: Option<Value>,
    pub discoverable: Option<bool>,
    pub capabilities: Option<Value>,
}

impl From<ApActor> for NewRemoteActor {
    fn from(actor: ApActor) -> NewRemoteActor {
        NewRemoteActor {
            context: serde_json::to_value(actor.context.unwrap()).unwrap(),
            kind: actor.kind.to_string(),
            ap_id: actor.id.unwrap(),
            name: actor.name.unwrap(),
            preferred_username: actor.preferred_username,
            summary: actor.summary.unwrap(),
            inbox: actor.inbox,
            outbox: actor.outbox,
            followers: actor.followers,
            following: actor.following,
            liked: actor.liked,
            public_key: serde_json::to_value(&actor.public_key).unwrap(),
            featured: actor.featured,
            featured_tags: actor.featured_tags,
            url: actor.url,
            manually_approves_followers: actor.manually_approves_followers,
            published: actor.published,
            tag: handle_option(serde_json::to_value(&actor.tag).unwrap()),
            attachment: handle_option(serde_json::to_value(&actor.attachment).unwrap()),
            endpoints: handle_option(serde_json::to_value(&actor.endpoints).unwrap()),
            icon: handle_option(serde_json::to_value(&actor.icon).unwrap()),
            image: handle_option(serde_json::to_value(&actor.image).unwrap()),
            also_known_as: handle_option(serde_json::to_value(&actor.also_known_as).unwrap()),
            discoverable: actor.discoverable,
            capabilities: handle_option(serde_json::to_value(&actor.capabilities).unwrap()),
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Debug)]
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
    pub featured: Option<String>,
    pub featured_tags: Option<String>,
    pub url: Option<String>,
    pub manually_approves_followers: Option<bool>,
    pub published: Option<String>,
    pub tag: Option<Value>,
    pub attachment: Option<Value>,
    pub endpoints: Option<Value>,
    pub icon: Option<Value>,
    pub image: Option<Value>,
    pub also_known_as: Option<Value>,
    pub discoverable: Option<bool>,
    pub capabilities: Option<Value>,
}
