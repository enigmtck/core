use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Utc;
use identicon_rs::color::RGB;
use identicon_rs::theme::HSLRange;
use identicon_rs::Identicon;
use orion::pwhash;
use rsa::{
    pkcs8::EncodePrivateKey, pkcs8::EncodePublicKey, pkcs8::LineEnding, RsaPrivateKey, RsaPublicKey,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::db::runner::DbRunner;
use crate::db::Db;
use crate::helper::get_ap_id_from_username;
use crate::models::actors::{
    create_or_update_actor, get_actor_by_username, Actor, ActorType, NewActor,
};
use crate::models::cache::Cache;
use crate::models::profiles::Profile;
use jdt_activity_pub::MaybeMultiple;
use jdt_activity_pub::{ApActor, ApCapabilities, ApContext, ApEndpoint, ApImage, ApPublicKey};

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

pub async fn authenticate<C: DbRunner>(
    conn: &C,
    username: String,
    password_str: String,
) -> Option<Profile> {
    log::debug!("AUTHENTICATING {username} {password_str}");
    let password = pwhash::Password::from_slice(password_str.clone().as_bytes()).ok()?;
    let profile = get_actor_by_username(conn, username.clone()).await.ok()?;
    let encoded_password_hash = profile.clone().ek_password?;
    let password_hash = pwhash::PasswordHash::from_encoded(&encoded_password_hash).ok()?;

    pwhash::hash_password_verify(&password_hash, &password).ok()?;

    profile.try_into().ok()
}

pub async fn verify_and_generate_password<C: DbRunner>(
    conn: &C,
    username: String,
    current_password: String,
    new_password: String,
) -> Option<String> {
    authenticate(conn, username, current_password).await?;

    let password = pwhash::Password::from_slice(new_password.as_bytes()).ok()?;
    let hash = pwhash::hash_password(&password, 3, 1 << 16).ok()?;

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
    pub kind: Option<ActorType>,
}

async fn generate_avatar(username: String) -> Result<String> {
    let filename = format!("{}.png", Uuid::new_v4());
    let media_dir = crate::MEDIA_DIR.as_str();
    let server_name = crate::SERVER_NAME.as_str();
    let local_path = format!("{media_dir}/avatars/{filename}");
    let handle = format!("@{username}@{server_name}");

    let security_background_colors = vec![
        RGB::from((54, 57, 63)), // Discord dark (professional)
        RGB::from((45, 48, 54)), // Darker variant
        RGB::from((48, 51, 57)), // Medium variant
        RGB::from((51, 54, 60)), // Lighter variant
    ];

    let security_theme = Arc::new(HSLRange::new(
        200.0,                      // hue_min: Blue-gray
        220.0,                      // hue_max: Blue-gray
        5.0,                        // saturation_min: Very desaturated
        20.0,                       // saturation_max: Subtle color
        15.0,                       // lightness_min: Very dark
        35.0,                       // lightness_max: Dark range
        security_background_colors, // background: Vec<RGB>
    )?);

    let cyberpunk_background_colors = vec![
        RGB::from((20, 20, 25)), // Dark blue-gray
        RGB::from((25, 20, 25)), // Dark magenta tint
        RGB::from((20, 25, 30)), // Dark cyan tint
        RGB::from((22, 22, 22)), // Neutral dark gray
    ];

    let cyberpunk_theme = Arc::new(HSLRange::new(
        180.0,                       // hue_min: Cyan
        320.0,                       // hue_max: Magenta
        40.0,                        // saturation_min: Vibrant colors
        70.0,                        // saturation_max: High saturation
        25.0,                        // lightness_min: Dark but visible
        40.0,                        // lightness_max: Bright enough for contrast
        cyberpunk_background_colors, // background: Vec<RGB>
    )?);

    Identicon::new(&handle)
        .set_border(50)
        .set_size(7)?
        .set_mirrored(true)
        .set_theme(security_theme)
        .save_image(&local_path)?;

    Ok(filename)
}

pub async fn create_user<C: DbRunner>(conn: &C, user: NewUser) -> Result<Actor> {
    let key_pair = get_key_pair();
    let owner = get_ap_id_from_username(user.username.clone());
    let server_name = crate::SERVER_NAME.as_str();
    let server_url = format!("https://{server_name}");
    let password = pwhash::Password::from_slice(user.password.as_bytes())?;
    let username = user.username.clone();
    let avatar = generate_avatar(username.clone()).await?;
    let hash = pwhash::hash_password(&password, 3, 1 << 16)?;

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
        as_public_key: json!(ApPublicKey {
            id: format!("{owner}#main-key"),
            owner: owner.clone(),
            public_key_pem: key_pair
                .public_key
                .to_public_key_pem(LineEnding::default())
                .unwrap(),
        }),
        ek_password: Some(hash.unprotected_as_encoded().to_string()),
        ek_client_public_key: user.client_public_key,
        ek_client_private_key: user.client_private_key,
        ek_olm_pickled_account: user.olm_pickled_account,
        ek_olm_pickled_account_hash: user.olm_pickled_account_hash,
        ek_olm_identity_key: user.olm_identity_key,
        ek_salt: user.salt,
        as_preferred_username: Some(username.clone()),
        as_inbox: format!("{owner}/inbox"),
        as_outbox: format!("{owner}/outbox"),
        as_followers: Some(format!("{owner}/followers")),
        as_following: Some(format!("{owner}/following")),
        as_liked: Some(format!("{owner}/liked")),
        ek_keys: Some(format!("{owner}/keys")),
        as_published: Some(Utc::now()),
        as_url: Some(json!(MaybeMultiple::from(format!(
            "{server_url}/@{username}"
        )))),
        as_endpoints: json!(ApEndpoint {
            shared_inbox: format!("{server_url}/inbox"),
        }),
        as_discoverable: true,
        ap_manually_approves_followers: false,
        ap_capabilities: json!(ApCapabilities {
            enigmatick_encryption: Some(true),
            ..Default::default()
        }),
        as_also_known_as: json!([]),
        as_tag: json!([]),
        as_id: owner,
        as_icon: {
            let mut image = ApImage::from(format!("{server_url}/media/avatars/{avatar}"));
            image.media_type = Some("image/png".to_string());
            json!(image)
        },
        as_image: json!("{}"),
        ek_webfinger: Some(format!("@{username}@{server_name}")),
        ek_avatar_filename: Some(avatar),
        ek_banner_filename: None,
        ek_checked_at: Utc::now(),
        ek_hashtags: json!([]),
        as_type: user.kind.unwrap_or(ActorType::Person),
        as_attachment: json!([]),
        as_context: Some(json!(ApContext::default())),
        as_featured: None,
        as_featured_tags: None,
    };

    let actor = create_or_update_actor(conn, new_profile).await?;
    ApActor::from(actor.clone()).cache(conn).await;

    Ok(actor)
}
