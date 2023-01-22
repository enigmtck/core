use std::collections::HashMap;

use crate::activity_pub::{retriever, ApActor, ApPublicKey};
use crate::db::{get_profile_by_username, Db};
use crate::models::profiles::Profile;
use rsa::pkcs1v15::{SigningKey, VerifyingKey};
use rsa::signature::{RandomizedSigner, Signature, Verifier};
use rsa::{pkcs8::DecodePrivateKey, pkcs8::DecodePublicKey, RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256};
use std::fmt::{self, Debug};
use std::time::SystemTime;
use url::Url;

#[derive(Clone)]
pub struct VerifyParams {
    pub profile: Profile,
    pub signature: String,
    pub request_target: String,
    pub host: String,
    pub date: String,
    pub digest: Option<String>,
    pub content_type: String,
}

fn build_verify_string(
    params: VerifyParams,
) -> (String, String, String, Option<String>, bool, Option<String>) {
    let mut signature_map = HashMap::<String, String>::new();

    let parts_re = regex::Regex::new(r#"(\w+)="(.+?)""#).unwrap();

    for cap in parts_re.captures_iter(&params.signature) {
        //log::debug!("re: {:#?}", &cap);
        signature_map.insert(cap[1].to_string(), cap[2].to_string());
    }

    log::debug!("map: {:#?}", signature_map);

    let key_id = signature_map.get("keyId").unwrap();
    let key_id_parts = key_id.split('#').collect::<Vec<&str>>();
    let ap_id = key_id_parts[0].to_string();
    let key_selector = key_id_parts[1].to_string();

    let local_pattern = format!(r#"(\w+://{}/user/(.+?))#(.+)"#, &*crate::SERVER_NAME);
    let local_re = regex::Regex::new(local_pattern.as_str()).unwrap();

    let mut local = false;
    let mut username = Option::<String>::None;
    let mut key_selector = Option::<String>::None;

    if let Some(captures) = local_re.captures(key_id) {
        local = true;
        username = Option::from(captures[2].to_string());
        key_selector = Option::from(captures[3].to_string());
        log::debug!("key_re: {:#?}", captures);
    }

    let mut verify_string = String::new();

    for part in signature_map.get("headers").unwrap().split(' ') {
        match part {
            "(request-target)" => {
                verify_string += &format!("(request-target): {}\n", params.request_target)
            }
            "host" => verify_string += &format!("host: {}\n", params.host),
            "date" => verify_string += &format!("date: {}\n", params.date),
            "digest" => verify_string += &format!("digest: {}\n", params.digest.clone().unwrap()),
            "content-type" => verify_string += &format!("content-type: {}\n", params.content_type),
            _ => (),
        }
    }

    log::debug!(
        "verify_string\n{}\nap_id: {:#?}\nkey_selector: {:#?}\nlocal: {}\nusername: {:#?}\n",
        verify_string,
        ap_id,
        key_selector,
        local,
        username
    );

    // (verify, signature, ap_id)
    (
        verify_string.trim_end().to_string(),
        signature_map.get("signature").unwrap().to_string(),
        ap_id,
        key_selector,
        local,
        username,
    )
}

pub async fn verify(conn: Db, params: VerifyParams) -> bool {
    let (verify_string, signature_str, ap_id, key_selector, local, username) =
        build_verify_string(params.clone());

    fn verify(public_key: RsaPublicKey, signature_str: String, verify_string: String) -> bool {
        let verifying_key: VerifyingKey<Sha256> = VerifyingKey::new_with_prefix(public_key);
        log::debug!("signature string: {}", signature_str);

        let s = base64::decode(signature_str.as_bytes()).unwrap();

        let signature: rsa::pkcs1v15::Signature = rsa::pkcs1v15::Signature::from(s);
        match verifying_key.verify(verify_string.as_bytes(), &signature) {
            Ok(_) => {
                log::debug!("signature verification successful");
                true
            }
            Err(_) => {
                log::debug!("signature verification failed");
                false
            }
        }
    }

    if local && key_selector == Some("client-key".to_string()) {
        if let Some(username) = username {
            if let Some(profile) = get_profile_by_username(&conn, username).await {
                if let Some(public_key) = profile.client_public_key {
                    if let Ok(public_key) = RsaPublicKey::from_public_key_pem(&public_key) {
                        verify(public_key, signature_str, verify_string)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else if let Some(actor) = retriever::get_actor(&conn, params.profile, ap_id).await {
        if let Some(public_key_value) = actor.0.public_key {
            if let Ok(public_key) = serde_json::from_value::<ApPublicKey>(public_key_value) {
                log::debug!("remote public key\n{}\n", public_key.public_key_pem);
                if let Ok(public_key) =
                    RsaPublicKey::from_public_key_pem(&public_key.public_key_pem)
                {
                    verify(public_key, signature_str, verify_string)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
}

// #[derive(Clone)]
// pub struct SignParams {
//     pub profile: Profile,
//     pub request_target: String,
//     pub host: String,
//     pub date: String,
//     pub digest: Option<String>,
// }

#[derive(Debug, Clone)]
pub enum Method {
    Get,
    Post,
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Clone)]
pub struct SignParams {
    pub profile: Profile,
    pub url: String,
    pub body: Option<String>,
    pub method: Method,
}

pub struct SignResponse {
    pub signature: String,
    pub date: String,
    pub digest: Option<String>,
}

pub fn sign(params: SignParams) -> SignResponse {
    // (request-target): post /users/justin/inbox
    // host: ser.endipito.us
    // date: Tue, 20 Dec 2022 22:02:48 GMT
    // digest: sha-256=uus37v4gf3z6ze+jtuyk+8xsT01FhYOi/rOoDfFV1u4=

    let digest = {
        if let Some(body) = params.body {
            let mut hasher = Sha256::new();
            hasher.update(body.as_bytes());
            let hashed = base64::encode(hasher.finalize());
            Option::from(format!("sha-256={}", hashed))
        } else {
            Option::None
        }
    };

    let url = Url::parse(&params.url).unwrap();
    let host = url.host().unwrap().to_string();
    let request_target = format!(
        "{} {}",
        params.method.to_string().to_lowercase(),
        url.path()
    );

    let now = SystemTime::now();
    let date = httpdate::fmt_http_date(now);

    let actor = ApActor::from(params.profile.clone());

    let private_key = RsaPrivateKey::from_pkcs8_pem(&params.profile.private_key).unwrap();
    let signing_key = SigningKey::<Sha256>::new_with_prefix(private_key);

    if let Some(digest) = digest {
        let structured_data = format!(
            "(request-target): {}\nhost: {}\ndate: {}\ndigest: {}",
            request_target, host, date, digest
        );

        log::debug!("\n{}", structured_data);

        let mut rng = rand::thread_rng();
        let signature = signing_key.sign_with_rng(&mut rng, structured_data.as_bytes());

        SignResponse {
            signature: format!(
                "keyId=\"{}\",headers=\"(request-target) host date digest\",signature=\"{}\"",
                actor.public_key.id,
                base64::encode(signature.as_bytes())
            ),
            date,
            digest: Option::from(digest),
        }
    } else {
        let structured_data = format!(
            "(request-target): {}\nhost: {}\ndate: {}\n",
            request_target, host, date
        );

        log::debug!("\n{}", structured_data);

        let mut rng = rand::thread_rng();
        let signature = signing_key.sign_with_rng(&mut rng, structured_data.as_bytes());

        SignResponse {
            signature: format!(
                "keyId=\"{}\",headers=\"(request-target) host date\",signature=\"{}\"",
                actor.public_key.id,
                base64::encode(signature.as_bytes())
            ),
            date,
            digest: Option::None,
        }
    }
}
