use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use jdt_activity_pub::ApAddress;
use serde::{Deserialize, Serialize};

use super::actors::Actor;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Profile {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub id: ApAddress,
    pub username: String,
    pub display_name: String,
    pub summary: Option<String>,
    pub public_key: String,
    pub client_public_key: Option<String>,
    pub avatar_filename: Option<String>,
    pub banner_filename: Option<String>,
    pub salt: Option<String>,
    pub client_private_key: Option<String>,
    pub olm_pickled_account: Option<String>,
    pub olm_pickled_account_hash: Option<String>,
    pub olm_identity_key: Option<String>,
    pub summary_markdown: Option<String>,
    pub followers: Option<String>,
    pub following: Option<String>,
}

impl TryFrom<Actor> for Profile {
    type Error = anyhow::Error;

    fn try_from(actor: Actor) -> Result<Self> {
        Ok(Profile {
            created_at: actor.created_at,
            updated_at: actor.updated_at,
            uuid: actor.ek_uuid.ok_or(anyhow!("no uuid"))?,
            id: actor.as_id.into(),
            username: actor.ek_username.ok_or(anyhow!("no username"))?,
            display_name: actor.as_name.ok_or(anyhow!("no name"))?,
            summary: actor.as_summary,
            summary_markdown: actor.ek_summary_markdown,
            public_key: actor.as_public_key.to_string(),
            client_public_key: actor.ek_client_public_key,
            client_private_key: actor.ek_client_private_key,
            avatar_filename: Some(
                actor
                    .ek_avatar_filename
                    .unwrap_or((*crate::DEFAULT_AVATAR).clone()),
            ),
            banner_filename: actor.ek_banner_filename,
            salt: actor.ek_salt,
            olm_pickled_account: actor.ek_olm_pickled_account,
            olm_pickled_account_hash: actor.ek_olm_pickled_account_hash,
            olm_identity_key: actor.ek_olm_identity_key,
            followers: actor.as_followers,
            following: actor.as_following,
        })
    }
}
