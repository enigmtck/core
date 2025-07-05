use std::collections::HashMap;

use crate::db::Db;
use crate::models::actors::{get_actor_by_key_id, get_actor_by_username, Actor};
use crate::{ASSIGNMENT_RE, LOCAL_USER_KEY_ID_RE};
use anyhow::anyhow;
use base64::{engine::general_purpose, engine::Engine as _};
use jdt_activity_pub::ApActor;
use rsa::pkcs1v15::{Signature, SigningKey};
use rsa::signature::{RandomizedSigner, SignatureEncoding, Verifier};
use rsa::{pkcs8::DecodePrivateKey, pkcs8::DecodePublicKey, RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt::{self, Debug};
use std::time::SystemTime;
use url::Url;

#[derive(Clone, Debug)]
pub struct VerifyMapParams {
    pub signature: String,
    pub request_target: String,
    pub host: String,
    pub date: String,
    pub digest: Option<String>,
    pub content_type: Option<String>,
    pub content_length: Option<String>,
    pub user_agent: Option<String>,
}

impl std::fmt::Display for VerifyMapParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Request[target: {}, host: {}, signature: {}, date: {}{}{}{}{}]",
            self.request_target,
            self.host,
            self.signature,
            self.date,
            self.digest
                .as_ref()
                .map(|d| format!(", digest: {d}"))
                .unwrap_or_default(),
            self.content_type
                .as_ref()
                .map(|ct| format!(", content-type: {ct}"))
                .unwrap_or_default(),
            self.content_length
                .as_ref()
                .map(|cl| format!(", content-length: {cl}"))
                .unwrap_or_default(),
            self.user_agent
                .as_ref()
                .map(|ua| format!(", user-agent: {ua}"))
                .unwrap_or_default()
        )
    }
}

#[derive(Clone, Debug)]
pub struct VerifyParams {
    pub verify_string: String,
    pub signature: String,
    pub key_id: String,
    pub key_selector: Option<String>,
    pub local: bool,
    pub signer_username: Option<String>,
}

impl std::fmt::Display for VerifyParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Verify[key: {}, {}{}{}]",
            self.key_id,
            if self.local { "local" } else { "remote" },
            self.key_selector
                .as_ref()
                .map(|ks| format!(", selector: {}", ks))
                .unwrap_or_default(),
            self.signer_username
                .as_ref()
                .map(|u| format!(", user: {}", u))
                .unwrap_or_default()
        )
    }
}

pub fn build_verify_string(params: VerifyMapParams) -> VerifyParams {
    let mut signature_map = HashMap::<String, String>::new();

    for cap in ASSIGNMENT_RE.captures_iter(&params.signature) {
        signature_map.insert(cap[1].to_string(), cap[2].to_string());
    }

    let key_id = signature_map
        .get("keyId")
        .expect("keyId not found in signature_map")
        .clone();

    let mut local = false;
    let mut signer_username = Option::<String>::None;
    let mut key_selector = Option::<String>::None;

    if let Some(captures) = LOCAL_USER_KEY_ID_RE.captures(&key_id) {
        local = true;
        signer_username = Some(captures[2].to_string());
        key_selector = Some(captures[3].to_string());
    }

    let headers = signature_map
        .get("headers")
        .expect("headers not found in signature_map");

    let verify_string = headers
        .split_whitespace()
        .map(|part| match part {
            "(request-target)" => format!("(request-target): {}", params.request_target),
            "host" => format!("host: {}", params.host),
            "date" => format!("date: {}", params.date),
            "digest" => format!("digest: {}", params.digest.clone().unwrap_or_default()),
            "content-type" => format!(
                "content-type: {}",
                params.content_type.clone().unwrap_or_default()
            ),
            "content-length" => format!(
                "content-length: {}",
                params.content_length.clone().unwrap_or_default()
            ),
            "user-agent" => format!(
                "user-agent: {}",
                params.user_agent.clone().unwrap_or_default()
            ),
            _ => String::new(),
        })
        .collect::<Vec<String>>()
        .join("\n");

    VerifyParams {
        verify_string,
        signature: signature_map
            .get("signature")
            .expect("signature not found in signature_map")
            .clone(),
        key_id,
        key_selector,
        local,
        signer_username,
    }
}

// Remote and Local need to pass back the base64-encoded hash so that the destination
// endpoint can verify it
#[derive(Clone, Debug)]
pub enum VerificationType {
    Remote((Box<ApActor>, Option<String>)),
    Local((Box<Actor>, Option<String>)),
    None,
    Deferred(Box<VerifyMapParams>),
}

#[derive(Debug)]
pub enum VerificationError {
    DecodeError(anyhow::Error),
    SignatureError(anyhow::Error),
    VerificationFailed(anyhow::Error),
    PublicKeyError(anyhow::Error),
    ActorNotFound(Box<VerifyMapParams>),
    ProfileNotFound,
    ClientKeyNotFound,
    NoKeyId,
}

impl std::fmt::Display for VerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationError::DecodeError(e) => write!(f, "failed to decode data: {}", e),
            VerificationError::SignatureError(e) => write!(f, "invalid signature: {}", e),
            VerificationError::VerificationFailed(e) => write!(f, "verification failed: {}", e),
            VerificationError::PublicKeyError(e) => write!(f, "invalid public key: {}", e),
            VerificationError::ActorNotFound(params) => {
                write!(f, "actor not found for params: {:?}", params)
            }
            VerificationError::ProfileNotFound => write!(f, "profile not found"),
            VerificationError::ClientKeyNotFound => write!(f, "client key not found"),
            VerificationError::NoKeyId => write!(f, "keyId not found in signature"),
        }
    }
}

pub fn verify_signature_crypto(
    public_key_pem: &str,
    signature_str: &str,
    verify_string: &str,
) -> Result<(), VerificationError> {
    let public_key = RsaPublicKey::from_public_key_pem(public_key_pem.trim_end())
        .map_err(|e| VerificationError::PublicKeyError(anyhow!(e)))?;
    let verifying_key = rsa::pkcs1v15::VerifyingKey::<Sha256>::new(public_key);

    let signature_bytes = general_purpose::STANDARD
        .decode(signature_str.as_bytes())
        .map_err(|e| VerificationError::DecodeError(anyhow!(e)))?;

    let signature = rsa::pkcs1v15::Signature::try_from(signature_bytes.as_slice())
        .map_err(|e| VerificationError::SignatureError(anyhow!(e)))?;

    verifying_key
        .verify(verify_string.as_bytes(), &signature)
        .map_err(|e| VerificationError::VerificationFailed(anyhow!(e)))
}

pub async fn verify(
    conn: &Db,
    params: VerifyMapParams,
) -> Result<VerificationType, VerificationError> {
    let verify_params = build_verify_string(params.clone());

    let VerifyParams {
        verify_string,
        signature: signature_str,
        key_id,
        key_selector,
        local,
        signer_username: username,
    } = verify_params.clone();

    // The old inner `verify` function is no longer needed.

    if local && key_selector == Some("client-key".to_string()) {
        let username = username.ok_or(VerificationError::ProfileNotFound)?;
        let profile = get_actor_by_username(Some(conn), username)
            .await
            .map_err(|_| VerificationError::ProfileNotFound)?;

        let public_key_pem = profile
            .ek_client_public_key
            .clone()
            .ok_or(VerificationError::ClientKeyNotFound)?;

        // Use the new helper function
        verify_signature_crypto(&public_key_pem, &signature_str, &verify_string)?;

        Ok(VerificationType::Local((Box::from(profile), params.digest)))
    } else if let Ok(actor) = get_actor_by_key_id(conn, key_id).await {
        let ap_actor = ApActor::from(actor.clone());
        let public_key_pem = ap_actor.clone().public_key.public_key_pem;

        // Use the new helper function
        verify_signature_crypto(&public_key_pem, &signature_str, &verify_string)?;
        Ok(VerificationType::Remote((
            Box::new(ap_actor),
            params.digest,
        )))
    } else {
        Err(VerificationError::ActorNotFound(params.into()))
    }
}

#[derive(Debug)]
pub enum SigningError {
    InvalidUrl,
    NoPrivateKey,
}

impl fmt::Display for SigningError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for SigningError {
    fn description(&self) -> &str {
        match self {
            SigningError::InvalidUrl => "URL is invalid",
            SigningError::NoPrivateKey => "Private key is missing or invalid",
        }
    }
}

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

#[derive(Clone, Debug)]
pub struct SignParams {
    pub profile: Actor,
    pub url: Url,
    pub body: Option<String>,
    pub method: Method,
}

impl std::fmt::Display for SignParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Sign[{} {} by {}{}]",
            self.method,
            self.url,
            self.profile.as_id,
            self.body
                .as_ref()
                .map(|b| format!(" with body ({} bytes)", b.len()))
                .unwrap_or_default()
        )
    }
}

pub struct SignResponse {
    pub signature: String,
    pub date: String,
    pub digest: Option<String>,
}

pub fn sign(params: SignParams) -> Result<SignResponse, SigningError> {
    let digest = compute_digest(&params.body);
    if let Some(host) = params.url.host() {
        let request_target = format_request_target(&params.method, &params.url);
        let date = httpdate::fmt_http_date(SystemTime::now());

        log::debug!("{params}");

        let actor = ApActor::from(params.profile.clone());
        let private_key = RsaPrivateKey::from_pkcs8_pem(
            &params
                .profile
                .ek_private_key
                .ok_or(SigningError::NoPrivateKey)?,
        )
        .unwrap();
        let signing_key = SigningKey::<Sha256>::new(private_key);
        let structured_data =
            construct_structured_data(&request_target, &host.to_string(), &date, &digest);
        let signature = compute_signature(&signing_key, &structured_data);
        let response_signature = format_response_signature(&actor, &signature, digest.is_some());

        Ok(SignResponse {
            signature: response_signature,
            date,
            digest,
        })
    } else {
        Err(SigningError::InvalidUrl)
    }
}

pub fn get_hash(bytes: Vec<u8>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hashed = hasher.finalize();
    general_purpose::STANDARD.encode(hashed)
}

fn compute_digest(body: &Option<String>) -> Option<String> {
    body.as_ref().map(|body| {
        let mut hasher = Sha256::new();
        hasher.update(body.as_bytes());
        let hashed = general_purpose::STANDARD.encode(hasher.finalize());
        format!("SHA-256={}", hashed)
    })
}

fn format_request_target(method: &Method, url: &Url) -> String {
    format!("{} {}", method.to_string().to_lowercase(), url.path())
}

fn construct_structured_data(
    request_target: &str,
    host: &str,
    date: &str,
    digest: &Option<String>,
) -> String {
    if let Some(ref digest) = digest {
        format!(
            "(request-target): {}\nhost: {}\ndate: {}\ndigest: {}",
            request_target, host, date, digest
        )
    } else {
        format!(
            "(request-target): {}\nhost: {}\ndate: {}",
            request_target, host, date
        )
    }
}

fn compute_signature(signing_key: &SigningKey<Sha256>, structured_data: &str) -> Signature {
    let mut rng = rand::thread_rng();
    signing_key.sign_with_rng(&mut rng, structured_data.as_bytes())
}

fn format_response_signature(actor: &ApActor, signature: &Signature, has_digest: bool) -> String {
    if has_digest {
        format!(
            "keyId=\"{}\",algorithm=\"rsa-sha256\",headers=\"(request-target) host date digest\",signature=\"{}\"",
            actor.public_key.id,
            general_purpose::STANDARD.encode(signature.to_bytes())
        )
    } else {
        format!(
            "keyId=\"{}\",algorithm=\"rsa-sha256\",headers=\"(request-target) host date\",signature=\"{}\"",
            actor.public_key.id,
            general_purpose::STANDARD.encode(signature.to_bytes())
        )
    }
}
