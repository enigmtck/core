use crate::activity_pub::{ApActivity, ApAnnounce, ApCreate, ApDelete, ApFollow, ApLike};
use crate::models::activities::ActivityType;
use crate::models::actors::ActorType;
use crate::models::objects::ObjectType;
use crate::schema::sql_types::{
    ActivityType as SqlActivityType, ActorType as SqlActorType, ObjectType as SqlObjectType,
};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::{Bool, Integer, Jsonb, Nullable, Text, Timestamptz};
use diesel::Queryable;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "pg")]
mod db_types {
    pub type DbObjectType = crate::models::objects::ObjectType;
    pub type DbActivityType = crate::models::activities::ActivityType;
    pub type DbActorType = crate::models::actors::ActorType;
}

#[cfg(feature = "sqlite")]
mod db_types {
    pub type DbObjectType = String;
    pub type DbActivityType = String;
    pub type DbActorType = String;
}

use db_types::*;

#[derive(Queryable, Serialize, Deserialize, Clone, Default, Debug, QueryableByName)]
pub struct CoalescedActivity {
    // Primary Activity Fields
    #[diesel(sql_type = Integer)]
    pub id: i32,

    #[diesel(sql_type = Timestamptz)]
    pub created_at: DateTime<Utc>,

    #[diesel(sql_type = Timestamptz)]
    pub updated_at: DateTime<Utc>,

    #[cfg_attr(feature = "pg", diesel(sql_type = SqlActivityType))]
    #[cfg_attr(feature = "sqlite", diesel(sql_type = Text))]
    pub kind: DbActivityType,

    //#[diesel(sql_type = SqlActivityType)]
    //pub kind: ActivityType,
    #[diesel(sql_type = Text)]
    pub uuid: String,

    #[diesel(sql_type = Text)]
    pub actor: String,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub ap_to: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub cc: Option<Value>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub target_activity_id: Option<i32>,

    #[diesel(sql_type = Nullable<Text>)]
    pub target_ap_id: Option<String>,

    #[diesel(sql_type = Bool)]
    pub revoked: bool,

    #[diesel(sql_type = Nullable<Text>)]
    pub ap_id: Option<String>,

    #[diesel(sql_type = Bool)]
    pub reply: bool,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub raw: Option<Value>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub target_object_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub actor_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub target_actor_id: Option<i32>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub log: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub instrument: Option<Value>,

    // Secondary Activity Fields
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub recursive_created_at: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub recursive_updated_at: Option<DateTime<Utc>>,

    #[cfg_attr(feature = "pg", diesel(sql_type = Nullable<SqlActivityType>))]
    #[cfg_attr(feature = "sqlite", diesel(sql_type = Nullable<Text>))]
    pub recursive_kind: Option<DbActivityType>,

    //#[diesel(sql_type = Nullable<SqlActivityType>)]
    //pub recursive_kind: Option<ActivityType>,
    #[diesel(sql_type = Nullable<Text>)]
    pub recursive_uuid: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub recursive_actor: Option<String>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub recursive_ap_to: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub recursive_cc: Option<Value>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub recursive_target_activity_id: Option<i32>,

    #[diesel(sql_type = Nullable<Text>)]
    pub recursive_target_ap_id: Option<String>,

    #[diesel(sql_type = Nullable<Bool>)]
    pub recursive_revoked: Option<bool>,

    #[diesel(sql_type = Nullable<Text>)]
    pub recursive_ap_id: Option<String>,

    #[diesel(sql_type = Nullable<Bool>)]
    pub recursive_reply: Option<bool>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub recursive_target_object_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub recursive_actor_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub recursive_target_actor_id: Option<i32>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub recursive_instrument: Option<Value>,

    // Object Fields
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub object_created_at: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub object_updated_at: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_uuid: Option<String>,

    #[cfg_attr(feature = "pg", diesel(sql_type = Nullable<SqlObjectType>))]
    #[cfg_attr(feature = "sqlite", diesel(sql_type = Nullable<Text>))]
    pub object_type: Option<DbObjectType>,

    //#[diesel(sql_type = Nullable<SqlObjectType>)]
    //pub object_type: Option<ObjectType>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub object_published: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_as_id: Option<String>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_url: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_to: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_cc: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_tag: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_attributed_to: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_in_reply_to: Option<Value>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_content: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_conversation: Option<String>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_attachment: Option<Value>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_summary: Option<String>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub object_end_time: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_one_of: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_any_of: Option<Value>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub object_voters_count: Option<i32>,

    #[diesel(sql_type = Nullable<Bool>)]
    pub object_sensitive: Option<bool>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_metadata: Option<Value>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub object_profile_id: Option<i32>,

    #[diesel(sql_type = Jsonb)]
    pub object_announcers: Value,

    #[diesel(sql_type = Jsonb)]
    pub object_likers: Value,

    #[diesel(sql_type = Jsonb)]
    pub object_attributed_to_profiles: Value,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_announced: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_liked: Option<String>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_instrument: Option<Value>,

    // Actor Fields
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub actor_created_at: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub actor_updated_at: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_uuid: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_username: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_summary_markdown: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_avatar_filename: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_banner_filename: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_private_key: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_password: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_client_public_key: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_client_private_key: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_salt: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_olm_pickled_account: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_olm_pickled_account_hash: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_olm_identity_key: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_webfinger: Option<String>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub actor_checked_at: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub actor_hashtags: Option<Value>,

    #[cfg_attr(feature = "pg", diesel(sql_type = Nullable<SqlActorType>))]
    #[cfg_attr(feature = "sqlite", diesel(sql_type = Nullable<Text>))]
    pub actor_type: Option<DbActorType>,

    //#[diesel(sql_type = Nullable<SqlActorType>)]
    //pub actor_type: Option<ActorType>,
    #[diesel(sql_type = Nullable<Jsonb>)]
    pub actor_context: Option<Value>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_as_id: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_name: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_preferred_username: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_summary: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_inbox: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_outbox: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_followers: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_following: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_liked: Option<String>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub actor_public_key: Option<Value>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_featured: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_featured_tags: Option<String>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub actor_url: Option<Value>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub actor_published: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub actor_tag: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub actor_attachment: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub actor_endpoints: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub actor_icon: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub actor_image: Option<Value>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub actor_also_known_as: Option<Value>,

    #[diesel(sql_type = Nullable<Bool>)]
    pub actor_discoverable: Option<bool>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub actor_capabilities: Option<Value>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_keys: Option<String>,

    #[diesel(sql_type = Nullable<Bool>)]
    pub actor_manually_approves_followers: Option<bool>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub actor_last_decrypted_activity: Option<DateTime<Utc>>,

    // Vault Fields
    #[diesel(sql_type = Nullable<Integer>)]
    pub vault_id: Option<i32>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub vault_created_at: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub vault_updated_at: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Text>)]
    pub vault_uuid: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub vault_owner_as_id: Option<String>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub vault_activity_id: Option<i32>,

    #[diesel(sql_type = Nullable<Text>)]
    pub vault_data: Option<String>,

    // Olm Session Fields
    #[diesel(sql_type = Nullable<Text>)]
    pub olm_data: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub olm_hash: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub olm_uuid: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub olm_conversation: Option<String>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub olm_created_at: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub olm_updated_at: Option<DateTime<Utc>>,

    #[diesel(sql_type = Nullable<Text>)]
    pub olm_owner: Option<String>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub olm_owner_id: Option<i32>,
}

impl TryFrom<CoalescedActivity> for ApActivity {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        match coalesced.kind {
            ActivityType::Create => ApCreate::try_from(coalesced).map(ApActivity::Create),
            ActivityType::Announce => ApAnnounce::try_from(coalesced).map(ApActivity::Announce),
            ActivityType::Like => ApLike::try_from(coalesced).map(|x| ApActivity::Like(x.into())),
            ActivityType::Delete => {
                ApDelete::try_from(coalesced).map(|x| ApActivity::Delete(x.into()))
            }
            ActivityType::Follow => ApFollow::try_from(coalesced).map(ApActivity::Follow),
            _ => {
                log::error!("Failed to match implemented Activity\n{coalesced:#?}");
                Err(anyhow!("Failed to match implemented Activity"))
            }
        }
    }
}
