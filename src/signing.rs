use std::collections::HashMap;

use crate::activity_pub::ApActor;
use crate::db::Db;
use crate::models::actors::{get_actor_by_key_id, get_actor_by_username, Actor};
use crate::{ASSIGNMENT_RE, LOCAL_USER_KEY_ID_RE};
use anyhow::anyhow;
use base64::{engine::general_purpose, engine::Engine as _};
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
    pub content_type: String,
    pub content_length: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Clone, Debug)]
pub struct VerifyParams {
    verify_string: String,
    signature: String,
    key_id: String,
    key_selector: Option<String>,
    local: bool,
    signer_username: Option<String>,
}

fn build_verify_string(params: VerifyMapParams) -> VerifyParams {
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
            "content-type" => format!("content-type: {}", params.content_type),
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

    log::debug!("VERIFY STRING: {verify_string}");

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

#[derive(Clone)]
pub enum VerificationType {
    Remote,
    Local(Box<Actor>),
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

    fn verify(
        public_key: &RsaPublicKey,
        signature_str: &str,
        verify_string: &str,
    ) -> Result<(), VerificationError> {
        let verifying_key = rsa::pkcs1v15::VerifyingKey::<Sha256>::new(public_key.clone());

        general_purpose::STANDARD
            .decode(signature_str.as_bytes())
            .map_err(|e| VerificationError::DecodeError(anyhow!(e)))
            .and_then(|signature_bytes| {
                rsa::pkcs1v15::Signature::try_from(signature_bytes.as_slice())
                    .map_err(|e| VerificationError::SignatureError(anyhow!(e)))
            })
            .and_then(|signature| {
                verifying_key
                    .verify(verify_string.as_bytes(), &signature)
                    .map_err(|e| VerificationError::VerificationFailed(anyhow!(e)))
            })
    }

    if local && key_selector == Some("client-key".to_string()) {
        let username = username.ok_or(VerificationError::ProfileNotFound)?;
        let profile = get_actor_by_username(conn, username)
            .await
            .ok_or(VerificationError::ProfileNotFound)?;

        let public_key = profile
            .ek_client_public_key
            .clone()
            .ok_or(VerificationError::ClientKeyNotFound)?;

        RsaPublicKey::from_public_key_pem(public_key.trim_end())
            .map_err(|e| VerificationError::PublicKeyError(anyhow!(e)))
            .and_then(|pk| verify(&pk, &signature_str, &verify_string))?;

        Ok(VerificationType::Local(Box::from(profile)))
    } else if let Ok(actor) = get_actor_by_key_id(conn, key_id).await {
        // I'm making a conscious choice here to limit accessibility to Actors that already have local records.
        // Because all we get is a KeyId in this exchange, and because KeyIds do not have a standard format
        // that lends itself to Actor ID discovery, trying to resolve to an unknown Actor ID is guesswork.
        // For example, Mastodon and Enigmatick use https://{server}/user(s)/{username}#main-key. GoToSocial
        // uses https://{server}/users/{username}/main-key. I could RegEx the variant, but it's clear that with
        // no real standard, I'd be chasing subjectivity across implementations.
        //
        // This approach should be okay with one exception: if we eventually integrate with a Relay of some sort
        // where we're getting messages from completely unknown Actors.
        RsaPublicKey::from_public_key_pem(
            ApActor::from(actor).public_key.public_key_pem.trim_end(),
        )
        .map_err(|e| VerificationError::PublicKeyError(anyhow!(e)))
        .and_then(|pk| verify(&pk, &signature_str, &verify_string))?;
        Ok(VerificationType::Remote)
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
        write!(f, "{:#?}", self)
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

        //log::debug!("SIGN {}, {host}, {request_target}, {date}", params.url);

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
