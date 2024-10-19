use anyhow::{anyhow, Result};
use chrono::Utc;
use orion::pwhash;
use rsa::{
    pkcs8::EncodePrivateKey, pkcs8::EncodePublicKey, pkcs8::LineEnding, RsaPrivateKey, RsaPublicKey,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::activity_pub::{
    ApCapabilities, ApContext, ApEndpoint, ApImage, ApImageType, ApPublicKey,
};
use crate::db::Db;
use crate::helper::get_ap_id_from_username;
use crate::models::actors::{
    create_or_update_actor, get_actor_by_username, Actor, ActorType, NewActor,
};
use crate::models::profiles::Profile;
use crate::models::to_serde;

struct KeyPair {
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
}

fn get_key_pair() -> KeyPair {
    let mut rng = rand::thread_rng();
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits).expect("failed");
    let public_key = RsaPublicKey::from(&private_key);

    KeyPair {
        private_key,
        public_key,
    }
}

pub async fn authenticate(conn: &Db, username: String, password_str: String) -> Option<Profile> {
    log::debug!("AUTHENTICATING {username} {password_str}");
    let password = pwhash::Password::from_slice(password_str.clone().as_bytes()).ok()?;
    let profile = get_actor_by_username(conn, username.clone()).await?;
    let encoded_password_hash = profile.clone().ek_password?;
    let password_hash = pwhash::PasswordHash::from_encoded(&encoded_password_hash).ok()?;

    pwhash::hash_password_verify(&password_hash, &password).ok()?;
    Some(profile.try_into().ok()?)
}

pub async fn verify_and_generate_password(
    conn: &Db,
    username: String,
    current_password: String,
    new_password: String,
) -> Option<String> {
    authenticate(conn, username, current_password).await?;

    let password = pwhash::Password::from_slice(new_password.as_bytes()).ok()?;
    // the example memory cost is 1<<16 (64MB); that taxes my system quite a bit,
    // so I'm using 8MB - this should be increased as available power permits

    let hash = pwhash::hash_password(&password, 3, 1 << 4).ok()?;

    Some(hash.unprotected_as_encoded().to_string())
}

#[derive(Deserialize, Debug, Clone)]
pub struct NewUser {
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub client_public_key: Option<String>,
    pub client_private_key: Option<String>,
    pub olm_pickled_account: Option<String>,
    pub olm_pickled_account_hash: Option<String>,
    pub olm_identity_key: Option<String>,
    pub salt: Option<String>,
}

pub async fn create_user(conn: Option<&Db>, user: NewUser) -> Result<Actor> {
    let key_pair = get_key_pair();
    let owner = get_ap_id_from_username(user.username.clone());
    let server = crate::SERVER_URL.clone();
    let password = pwhash::Password::from_slice(user.password.as_bytes())?;
    let username = user.username.clone();

    // the example memory cost is 1<<16 (64MB); that taxes my system quite a bit,
    // so I'm using 8MB - this should be increased as available power permits
    let hash = pwhash::hash_password(&password, 3, 1 << 4)?;
    let new_profile = NewActor {
        ek_uuid: Some(Uuid::new_v4().to_string()),
        ek_username: Some(username.clone()),
        as_name: Some(user.display_name),
        as_summary: None,
        ek_summary_markdown: None,
        ek_private_key: Some(
            key_pair
                .private_key
                .to_pkcs8_pem(LineEnding::default())
                .unwrap()
                .to_string(),
        ),
        as_public_key: to_serde(&Some(ApPublicKey {
            id: format!("{owner}#main-key"),
            owner: owner.clone(),
            public_key_pem: key_pair
                .public_key
                .to_public_key_pem(LineEnding::default())
                .unwrap(),
        }))
        .ok_or(anyhow!("failed to initialize public key"))?,
        ek_password: Some(hash.unprotected_as_encoded().to_string()),
        ek_client_public_key: user.client_public_key,
        ek_client_private_key: user.client_private_key,
        ek_olm_pickled_account: user.olm_pickled_account,
        ek_olm_pickled_account_hash: user.olm_pickled_account_hash,
        ek_olm_identity_key: user.olm_identity_key,
        ek_salt: user.salt,
        as_preferred_username: Some(owner.clone()),
        as_inbox: format!("{owner}/inbox"),
        as_outbox: format!("{owner}/outbox"),
        as_followers: Some(format!("{owner}/followers")),
        as_following: Some(format!("{owner}/following")),
        as_liked: Some(format!("{owner}/liked")),
        as_published: Some(Utc::now()),
        as_url: Some(format!("{server}/@{username}")),
        as_endpoints: to_serde(&Some(ApEndpoint {
            shared_inbox: format!("{server}/inbox"),
        }))
        .ok_or(anyhow!("failed to initialize endpoints"))?,
        as_discoverable: true,
        ap_manually_approves_followers: false,
        ap_capabilities: to_serde(&Some(ApCapabilities {
            accepts_chat_messages: Some(false),
            enigmatick_encryption: Some(true),
        }))
        .ok_or(anyhow!("failed to initialize capabilities"))?,
        as_also_known_as: json!([]),
        as_tag: json!([]),
        as_id: owner,
        as_icon: to_serde(&Some(ApImage {
            url: format!("{server}/media/avatars/default.png"),
            kind: ApImageType::Image,
            media_type: Some("png".to_string()),
        }))
        .ok_or(anyhow!("failed to initialize image"))?,
        as_image: json!("{}"),
        ek_webfinger: Some(format!("@{username}@{server}")),
        ek_avatar_filename: None,
        ek_banner_filename: None,
        ek_checked_at: Utc::now(),
        ek_hashtags: json!([]),
        as_type: ActorType::Person,
        as_attachment: json!([]),
        as_context: to_serde(&Some(ApContext::default())),
        as_featured: None,
        as_featured_tags: None,
    };

    create_or_update_actor(conn, new_profile).await
}
