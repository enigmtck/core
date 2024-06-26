use orion::pwhash;
use rsa::{
    pkcs8::EncodePrivateKey, pkcs8::EncodePublicKey, pkcs8::LineEnding, RsaPrivateKey, RsaPublicKey,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::db::Db;
use crate::models::profiles::{create_profile, get_profile_by_username, NewProfile, Profile};

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
    let profile = get_profile_by_username(conn.into(), username.clone()).await?;
    let encoded_password_hash = profile.clone().password?;
    let password_hash = pwhash::PasswordHash::from_encoded(&encoded_password_hash).ok()?;

    if pwhash::hash_password_verify(&password_hash, &password).is_ok() {
        Some(profile)
    } else {
        log::debug!("hash_password_verify failed {username} {password_str}");
        None
    }
}

pub async fn verify_and_generate_password(
    conn: &Db,
    username: String,
    current_password: String,
    new_password: String,
) -> Option<String> {
    if let Some(_profile) = authenticate(conn, username, current_password).await {
        if let Ok(password) = pwhash::Password::from_slice(new_password.as_bytes()) {
            // the example memory cost is 1<<16 (64MB); that taxes my system quite a bit,
            // so I'm using 8MB - this should be increased as available power permits
            if let Ok(hash) = pwhash::hash_password(&password, 3, 1 << 4) {
                Some(hash.unprotected_as_encoded().to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
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

pub async fn create_user(conn: Option<&Db>, user: NewUser) -> Option<Profile> {
    let key_pair = get_key_pair();

    if let Ok(password) = pwhash::Password::from_slice(user.password.as_bytes()) {
        // the example memory cost is 1<<16 (64MB); that taxes my system quite a bit,
        // so I'm using 8MB - this should be increased as available power permits
        if let Ok(hash) = pwhash::hash_password(&password, 3, 1 << 4) {
            let new_profile = NewProfile {
                uuid: Uuid::new_v4().to_string(),
                username: user.username,
                display_name: user.display_name,
                summary: None,
                summary_markdown: None,
                private_key: key_pair
                    .private_key
                    .to_pkcs8_pem(LineEnding::default())
                    .unwrap()
                    .to_string(),
                public_key: key_pair
                    .public_key
                    .to_public_key_pem(LineEnding::default())
                    .unwrap(),
                password: Some(hash.unprotected_as_encoded().to_string()),
                client_public_key: user.client_public_key,
                client_private_key: user.client_private_key,
                olm_pickled_account: user.olm_pickled_account,
                olm_pickled_account_hash: user.olm_pickled_account_hash,
                olm_identity_key: user.olm_identity_key,
                salt: user.salt,
            };

            create_profile(conn, new_profile).await
        } else {
            None
        }
    } else {
        None
    }
}
