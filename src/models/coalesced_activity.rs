use crate::models::activities::ActivityType;
use crate::models::from_serde;
use crate::models::objects::ObjectType;
use crate::schema::sql_types::{
    ActivityType as SqlActivityType, ActorType as SqlActorType, ObjectType as SqlObjectType,
};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::{Bool, Integer, Jsonb, Nullable, Text, Timestamptz};
use diesel::Queryable;
use jdt_activity_pub::{
    ApActivity, ApAddress, ApAnnounce, ApArticle, ApContext, ApCreate, ApDateTime, ApDelete,
    ApDeleteType, ApFollow, ApFollowType, ApInstrument, ApLike, ApLikeType, ApNote, ApObject,
    ApQuestion, ApUrl, Ephemeral, MaybeReference,
};
use jdt_activity_pub::{ApTimelineObject, MaybeMultiple};
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

    // #[diesel(sql_type = Nullable<Jsonb>)]
    // pub raw: Option<Value>,
    #[diesel(sql_type = Nullable<Integer>)]
    pub target_object_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub actor_id: Option<i32>,

    #[diesel(sql_type = Nullable<Integer>)]
    pub target_actor_id: Option<i32>,

    // #[diesel(sql_type = Nullable<Jsonb>)]
    // pub log: Option<Value>,
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

    #[diesel(sql_type = Nullable<Text>)]
    pub object_name: Option<String>,

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

    #[diesel(sql_type = Nullable<Text>)]
    pub object_content: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_conversation: Option<String>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_attachment: Option<Value>,

    #[diesel(sql_type = Nullable<Text>)]
    pub object_summary: Option<String>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_preview: Option<Value>,

    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub object_start_time: Option<DateTime<Utc>>,

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

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub object_in_reply_to: Option<Value>,

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

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub actor_featured: Option<Value>,

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

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_mls_credentials: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_mls_storage: Option<String>,

    #[diesel(sql_type = Nullable<Text>)]
    pub actor_mls_storage_hash: Option<String>,

    #[diesel(sql_type = Nullable<Jsonb>)]
    pub actor_muted_terms: Option<Value>,
    // // Vault Fields
    // #[diesel(sql_type = Nullable<Integer>)]
    // pub vault_id: Option<i32>,

    // #[diesel(sql_type = Nullable<Timestamptz>)]
    // pub vault_created_at: Option<DateTime<Utc>>,

    // #[diesel(sql_type = Nullable<Timestamptz>)]
    // pub vault_updated_at: Option<DateTime<Utc>>,

    // #[diesel(sql_type = Nullable<Text>)]
    // pub vault_uuid: Option<String>,

    // #[diesel(sql_type = Nullable<Text>)]
    // pub vault_owner_as_id: Option<String>,

    // #[diesel(sql_type = Nullable<Integer>)]
    // pub vault_activity_id: Option<i32>,

    // #[diesel(sql_type = Nullable<Text>)]
    // pub vault_data: Option<String>,

    // // MlsGroupId Fields
    // #[diesel(sql_type = Nullable<Integer>)]
    // pub mls_group_id_id: Option<i32>,

    // #[diesel(sql_type = Nullable<Timestamptz>)]
    // pub mls_group_id_created_at: Option<DateTime<Utc>>,

    // #[diesel(sql_type = Nullable<Timestamptz>)]
    // pub mls_group_id_updated_at: Option<DateTime<Utc>>,

    // #[diesel(sql_type = Nullable<Text>)]
    // pub mls_group_id_uuid: Option<String>,

    // #[diesel(sql_type = Nullable<Integer>)]
    // pub mls_group_id_actor_id: Option<i32>,

    // #[diesel(sql_type = Nullable<Text>)]
    // pub mls_group_id_conversation: Option<String>,

    // #[diesel(sql_type = Nullable<Text>)]
    // pub mls_group_id_mls_group: Option<String>,
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

impl TryFrom<CoalescedActivity> for ApAnnounce {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        let object = match coalesced.clone().object_type.ok_or_else(|| {
            log::error!("ObjectType appears to be None: {:?}", coalesced.clone());
            anyhow::anyhow!("object_type is None")
        })? {
            ObjectType::Note => Ok(ApObject::Note(ApNote::try_from(coalesced.clone())?).into()),
            ObjectType::Article => {
                Ok(ApObject::Article(ApArticle::try_from(coalesced.clone())?).into())
            }
            ObjectType::EncryptedNote => {
                Ok(ApObject::Note(ApNote::try_from(coalesced.clone())?).into())
            }
            ObjectType::Question => {
                Ok(ApObject::Question(ApQuestion::try_from(coalesced.clone())?).into())
            }
            _ => {
                log::error!("Invalid ObjectType: {:?}", coalesced.clone());
                Err(anyhow!("invalid type"))
            }
        }?;

        let kind = coalesced.kind.clone().try_into()?;
        let actor = ApAddress::Address(coalesced.actor.clone());
        let id = coalesced.ap_id.clone();
        let context = Some(ApContext::default());
        let to = coalesced.clone().ap_to.into();
        let cc = coalesced.clone().cc.into();
        let published = coalesced.created_at.into();
        let ephemeral = Some(Ephemeral {
            created_at: Some(coalesced.created_at),
            updated_at: Some(coalesced.updated_at),
            ..Default::default()
        });

        Ok(ApAnnounce {
            context,
            kind,
            actor,
            id,
            object,
            to,
            cc,
            published,
            ephemeral,
        })
    }
}

impl TryFrom<CoalescedActivity> for ApCreate {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        let object = match coalesced.clone().object_type.ok_or_else(|| {
            log::error!("ObjectType appears to be None: {:?}", coalesced.clone());
            anyhow::anyhow!("object_type is None")
        })? {
            ObjectType::Note => Ok(ApObject::Note(ApNote::try_from(coalesced.clone())?).into()),
            ObjectType::Article => {
                Ok(ApObject::Article(ApArticle::try_from(coalesced.clone())?).into())
            }
            ObjectType::EncryptedNote => {
                Ok(ApObject::Note(ApNote::try_from(coalesced.clone())?).into())
            }
            ObjectType::Question => {
                Ok(ApObject::Question(ApQuestion::try_from(coalesced.clone())?).into())
            }
            _ => {
                log::error!("Invalid ObjectType: {:?}", coalesced.clone());
                Err(anyhow!("invalid type"))
            }
        }?;
        let kind = coalesced.kind.clone().try_into()?;
        let actor = ApAddress::Address(coalesced.actor.clone());
        let id = coalesced.ap_id.clone();
        let context = Some(ApContext::default());
        let to = coalesced.ap_to.clone().into();
        let cc = coalesced.cc.clone().into();
        let signature = None;
        let published = Some(coalesced.created_at.into());
        let ephemeral = Some(Ephemeral {
            created_at: Some(coalesced.created_at),
            updated_at: Some(coalesced.updated_at),
            ..Default::default()
        });

        let mut instrument: MaybeMultiple<ApInstrument> = coalesced.instrument.clone().into();

        if let Ok(instruments) = Vec::<ApInstrument>::try_from(coalesced) {
            instrument = instrument.extend(instruments);
        }

        Ok(ApCreate {
            context,
            kind,
            actor,
            id,
            object,
            to,
            cc,
            signature,
            published,
            ephemeral,
            instrument,
        })
    }
}

impl TryFrom<CoalescedActivity> for ApDelete {
    type Error = anyhow::Error;

    fn try_from(activity: CoalescedActivity) -> Result<Self, Self::Error> {
        if !activity.kind.is_delete() {
            return Err(anyhow!("Not a Delete Activity"));
        }

        Ok(ApDelete {
            context: Some(ApContext::default()),
            kind: ApDeleteType::default(),
            actor: activity.actor.into(),
            id: activity.ap_id,
            to: activity.ap_to.into(),
            cc: activity.cc.into(),
            object: activity
                .object_as_id
                .ok_or(anyhow!("no object_as_id"))?
                .into(),
            signature: None,
        })
    }
}

impl TryFrom<CoalescedActivity> for ApFollow {
    type Error = anyhow::Error;

    fn try_from(activity: CoalescedActivity) -> Result<Self, Self::Error> {
        if activity.kind.is_follow() {
            Ok(ApFollow {
                context: Some(ApContext::default()),
                kind: ApFollowType::default(),
                actor: activity.actor.into(),
                id: Some(activity.ap_id.ok_or(anyhow!("no follow as_id found"))?),
                to: activity.ap_to.into(),
                cc: activity.cc.into(),
                object: activity
                    .object_as_id
                    .ok_or(anyhow!("no object_as_id"))?
                    .into(),
            })
        } else {
            log::error!("Not a Follow Activity");
            Err(anyhow!("Not a Follow Activity"))
        }
    }
}

impl TryFrom<CoalescedActivity> for ApLike {
    type Error = anyhow::Error;

    fn try_from(activity: CoalescedActivity) -> Result<Self, Self::Error> {
        if !activity.kind.is_like() {
            return Err(anyhow!("Not a Like Activity"));
        }

        Ok(ApLike {
            context: Some(ApContext::default()),
            kind: ApLikeType::default(),
            actor: activity.actor.into(),
            id: activity.ap_id,
            to: activity.ap_to.into(),
            object: activity
                .object_as_id
                .ok_or(anyhow!("no object_as_id"))?
                .into(),
        })
    }
}

impl TryFrom<CoalescedActivity> for ApNote {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        let kind = coalesced
            .object_type
            .ok_or_else(|| anyhow::anyhow!("object_type is None"))?
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to convert Note object_type: {}", e))?;

        let id = coalesced.object_as_id;
        let name = coalesced.object_name;

        let url = coalesced
            .object_url
            .and_then(from_serde::<MaybeMultiple<ApUrl>>)
            .unwrap_or(MaybeMultiple::None);
        let to = coalesced
            .object_to
            .and_then(from_serde)
            .ok_or_else(|| anyhow::anyhow!("object_to is None"))?;
        let cc: MaybeMultiple<ApAddress> = coalesced.object_cc.into();
        let tag = coalesced.object_tag.into();
        let attributed_to = coalesced
            .object_attributed_to
            .and_then(from_serde)
            .ok_or_else(|| anyhow::anyhow!("object_attributed_to is None"))?;

        let in_reply_to: MaybeMultiple<MaybeReference<ApTimelineObject>> =
            coalesced.object_in_reply_to.into();
        let content = coalesced.object_content;
        let conversation = coalesced.object_conversation;
        let attachment = coalesced.object_attachment.into();
        let summary = coalesced.object_summary;
        let sensitive = coalesced.object_sensitive;
        let published = coalesced
            .object_published
            .ok_or_else(|| anyhow::anyhow!("object_published is None"))?
            .into();

        // from_serde now includes enhanced error reporting
        let announces = from_serde(coalesced.object_announcers.clone());
        let likes = from_serde(coalesced.object_likers.clone());

        let ephemeral = Some(Ephemeral {
            metadata: coalesced.object_metadata.and_then(from_serde),
            announces,
            likes,
            announced: coalesced.object_announced,
            liked: coalesced.object_liked,
            attributed_to: from_serde(coalesced.object_attributed_to_profiles),
            ..Default::default()
        });

        let instrument = coalesced.object_instrument.into();

        Ok(ApNote {
            kind,
            id,
            name,
            url,
            to,
            cc,
            tag,
            attributed_to,
            in_reply_to,
            content,
            conversation,
            attachment,
            summary,
            sensitive,
            published,
            ephemeral,
            instrument,
            ..Default::default()
        })
    }
}

impl TryFrom<CoalescedActivity> for ApArticle {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        let kind = coalesced
            .object_type
            .ok_or_else(|| anyhow::anyhow!("object_type is None"))?
            .to_string()
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to convert Article object_type: {}", e))?;

        let id = coalesced.object_as_id;
        let name = coalesced.object_name;
        let url = coalesced
            .object_url
            .and_then(from_serde::<MaybeMultiple<ApUrl>>)
            .unwrap_or(MaybeMultiple::None);
        let to = coalesced
            .object_to
            .and_then(from_serde)
            .ok_or_else(|| anyhow::anyhow!("object_to is None"))?;
        let cc: MaybeMultiple<ApAddress> = coalesced.object_cc.into();
        let tag = coalesced.object_tag.into();
        let attributed_to = coalesced
            .object_attributed_to
            .and_then(from_serde)
            .ok_or_else(|| anyhow::anyhow!("object_attributed_to is None"))?;
        let in_reply_to: MaybeMultiple<MaybeReference<ApTimelineObject>> =
            coalesced.object_in_reply_to.into();
        let content = coalesced.object_content;
        let attachment = coalesced.object_attachment.into();
        let summary = coalesced.object_summary;
        let preview = coalesced.object_preview.into();
        let sensitive = coalesced.object_sensitive;
        let published = coalesced
            .object_published
            .ok_or_else(|| anyhow::anyhow!("object_published is None"))?
            .into();
        let ephemeral = Some(Ephemeral {
            metadata: coalesced.object_metadata.and_then(from_serde),
            announces: from_serde(coalesced.object_announcers),
            likes: from_serde(coalesced.object_likers),
            announced: coalesced.object_announced,
            liked: coalesced.object_liked,
            attributed_to: from_serde(coalesced.object_attributed_to_profiles),
            ..Default::default()
        });
        let instrument = coalesced.object_instrument.into();

        Ok(ApArticle {
            kind,
            id,
            name,
            url,
            to,
            cc,
            tag,
            attributed_to,
            in_reply_to,
            content,
            attachment,
            preview,
            summary,
            sensitive,
            published,
            ephemeral,
            instrument,
            ..Default::default()
        })
    }
}

impl TryFrom<CoalescedActivity> for ApQuestion {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        let kind = coalesced
            .object_type
            .ok_or_else(|| anyhow::anyhow!("object_type is None"))?
            .to_string()
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to convert Question object_type: {}", e))?;

        let id = coalesced.object_as_id;
        let url = coalesced
            .object_url
            .and_then(from_serde::<MaybeMultiple<ApUrl>>)
            .unwrap_or(MaybeMultiple::None);
        let to = coalesced
            .object_to
            .and_then(from_serde)
            .ok_or_else(|| anyhow::anyhow!("object_to is None"))?;
        let cc: MaybeMultiple<ApAddress> = coalesced.object_cc.into();
        let tag = coalesced.object_tag.into();
        let attributed_to = coalesced
            .object_attributed_to
            .and_then(from_serde)
            .ok_or_else(|| anyhow::anyhow!("object_attributed_to is None"))?;
        let in_reply_to: MaybeMultiple<MaybeReference<ApTimelineObject>> =
            coalesced.object_in_reply_to.into();
        let content = coalesced.object_content;
        let conversation = coalesced.object_conversation;
        let attachment = coalesced.object_attachment.into();
        let summary = coalesced.object_summary;
        let sensitive = coalesced.object_sensitive;
        let published = coalesced.object_published.map(ApDateTime::from);
        let _start_time = coalesced.object_start_time.map(ApDateTime::from);
        let end_time = coalesced.object_end_time.map(ApDateTime::from);
        let one_of = coalesced.object_one_of.into();
        let any_of = coalesced.object_any_of.into();
        let voters_count = coalesced.object_voters_count;
        let ephemeral = Some(Ephemeral {
            metadata: coalesced.object_metadata.and_then(from_serde),
            announces: from_serde(coalesced.object_announcers),
            likes: from_serde(coalesced.object_likers),
            announced: coalesced.object_announced,
            liked: coalesced.object_liked,
            attributed_to: from_serde(coalesced.object_attributed_to_profiles),
            ..Default::default()
        });

        Ok(ApQuestion {
            kind,
            id,
            url,
            to,
            cc,
            tag,
            attributed_to,
            in_reply_to,
            content,
            conversation,
            attachment,
            summary,
            sensitive,
            published,
            //start_time,
            end_time,
            one_of,
            any_of,
            voters_count,
            ephemeral,
            ..Default::default()
        })
    }
}

impl TryFrom<CoalescedActivity> for Vec<ApInstrument> {
    type Error = anyhow::Error;

    fn try_from(_coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        //let mut instruments: Vec<ApInstrument> = vec![];

        // if coalesced.vault_data.is_some() {
        //     instruments.push(ApInstrument {
        //         kind: ApInstrumentType::VaultItem,
        //         id: Some(get_instrument_as_id_from_uuid(
        //             coalesced
        //                 .vault_uuid
        //                 .clone()
        //                 .ok_or(anyhow!("VaultItem must have a UUID"))?,
        //         )),
        //         content: Some(
        //             coalesced
        //                 .vault_data
        //                 .ok_or(anyhow!("VaultItem must have content"))?,
        //         ),
        //         uuid: Some(
        //             coalesced
        //                 .vault_uuid
        //                 .ok_or(anyhow!("VaultItem must have a UUID"))?,
        //         ),
        //         hash: None,
        //         name: None,
        //         url: None,
        //         mutation_of: None,
        //         conversation: None,
        //         activity: None,
        //     });
        // }

        // if coalesced.mls_group_id_mls_group.is_some() {
        //     instruments.push(ApInstrument {
        //         kind: ApInstrumentType::MlsGroupId,
        //         id: Some(get_instrument_as_id_from_uuid(
        //             coalesced
        //                 .mls_group_id_uuid
        //                 .clone()
        //                 .ok_or(anyhow!("MlsGroupId must have a UUID"))?,
        //         )),
        //         content: Some(
        //             coalesced
        //                 .mls_group_id_mls_group
        //                 .ok_or(anyhow!("MlsGroupId must have Data"))?,
        //         ),
        //         uuid: Some(
        //             coalesced
        //                 .mls_group_id_uuid
        //                 .ok_or(anyhow!("MlsGroupId must have a UUID"))?,
        //         ),
        //         hash: None,
        //         name: None,
        //         url: None,
        //         mutation_of: None,
        //         conversation: Some(
        //             coalesced
        //                 .mls_group_id_conversation
        //                 .ok_or(anyhow!("MlsGroupId must have a conversation"))?,
        //         ),
        //         activity: None,
        //     });
        // }

        //Ok(instruments)
        Ok(vec![])
    }
}
