use crate::{
    blocklist::Permitted,
    models::{
        actors::{get_actor_by_key_id_axum, get_actor_by_username_axum},
        instances::{create_or_update_instance_axum, Instance},
    },
    server::AppState,
    signing::{
        build_verify_string, verify_signature_crypto, Signed, VerificationError, VerificationType,
        VerifyMapParams, VerifyParams,
    },
    ASSIGNMENT_RE, DOMAIN_RE,
};
use anyhow::{anyhow, Result};
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use deadpool_diesel::postgres::Object as DbConnection;
use jdt_activity_pub::ApActor;
use serde_json::json;
use std::collections::HashMap;
use std::ops::Deref;

fn get_header(parts: &Parts, header_name: &str) -> Option<String> {
    parts
        .headers
        .get(header_name)
        .and_then(|val| val.to_str().ok())
        .map(|s| s.to_string())
}

// 1. Define the new wrapper struct for the Axum extractor.
#[derive(Debug)]
pub struct AxumSigned(pub Signed);

// 2. Implement Deref to allow calling Signed's methods on AxumSigned.
impl Deref for AxumSigned {
    type Target = Signed;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

async fn update_instance_axum(conn: &DbConnection, signature: String) -> Result<Instance> {
    let mut signature_map = HashMap::<String, String>::new();
    for cap in ASSIGNMENT_RE.captures_iter(&signature) {
        signature_map.insert(cap[1].to_string(), cap[2].to_string());
    }
    let key_id = signature_map
        .get("keyId")
        .ok_or(anyhow!("keyId not found in signature_map"))?;
    let domain_name = DOMAIN_RE
        .captures(key_id)
        .ok_or(anyhow!("failed to retrieve key_id"))?[1]
        .to_string();

    let actor = get_actor_by_key_id_axum(conn, key_id.clone()).await.ok();
    let shared_inbox = actor.and_then(|actor| {
        ApActor::from(actor)
            .endpoints
            .map(|endpoints| endpoints.shared_inbox)
    });

    create_or_update_instance_axum(conn, (domain_name, shared_inbox).into()).await
}

async fn verify_axum(
    conn: &DbConnection,
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

    if local && key_selector == Some("client-key".to_string()) {
        let username = username.ok_or(VerificationError::ProfileNotFound)?;
        let profile = get_actor_by_username_axum(conn, username)
            .await
            .map_err(|_| VerificationError::ProfileNotFound)?;

        let public_key_pem = profile
            .ek_client_public_key
            .clone()
            .ok_or(VerificationError::ClientKeyNotFound)?;

        verify_signature_crypto(&public_key_pem, &signature_str, &verify_string)?;

        Ok(VerificationType::Local((Box::from(profile), params.digest)))
    } else if let Ok(actor) = get_actor_by_key_id_axum(conn, key_id).await {
        let ap_actor = ApActor::from(actor.clone());
        let public_key_pem = ap_actor.clone().public_key.public_key_pem;

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
pub enum SignedRejection {
    SignatureInvalid,
    MultipleSignatures,
    DatabaseUnavailable,
}

impl IntoResponse for SignedRejection {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            SignedRejection::SignatureInvalid => (StatusCode::BAD_REQUEST, "Invalid Signature"),
            SignedRejection::MultipleSignatures => {
                (StatusCode::BAD_REQUEST, "Multiple Signatures Provided")
            }
            SignedRejection::DatabaseUnavailable => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database unavailable")
            }
        };
        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}

impl FromRequestParts<AppState> for AxumSigned {
    type Rejection = SignedRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let conn = state
            .db_pool
            .get()
            .await
            .map_err(|_| SignedRejection::DatabaseUnavailable)?;

        let method = parts.method.to_string();
        let host = (*crate::SERVER_NAME).clone();
        let path = parts.uri.path().to_string();
        let path = path.trim_end_matches('&');
        let request_target = format!("{} {}", method.to_lowercase(), path);

        let date = match get_header(parts, "date").or_else(|| get_header(parts, "enigmatick-date"))
        {
            Some(val) => val,
            None => return Ok(AxumSigned(Signed(false, VerificationType::None))),
        };

        let digest = get_header(parts, "digest");
        let user_agent = get_header(parts, "user-agent");
        let content_length = get_header(parts, "content-length");
        let content_type = get_header(parts, "content-type");

        let signature_vec: Vec<_> = parts.headers.get_all("signature").iter().collect();

        match signature_vec.len() {
            0 => Ok(AxumSigned(Signed(false, VerificationType::None))),
            1 => {
                let signature = signature_vec[0].to_str().unwrap_or("").to_string();

                let verify_params = VerifyMapParams {
                    signature: signature.clone(),
                    request_target,
                    host,
                    date,
                    digest,
                    content_type,
                    content_length,
                    user_agent,
                };

                log::debug!("{verify_params}");

                match verify_axum(&conn, verify_params.clone()).await {
                    Ok(t) => {
                        log::debug!("Signature verification successful");
                        let _ = update_instance_axum(&conn, signature).await;
                        Ok(AxumSigned(Signed(true, t)))
                    }
                    Err(e) => match e {
                        VerificationError::ActorNotFound(ref verify_map_params) => {
                            log::debug!("Signature verification deferred: {e}");
                            Ok(AxumSigned(Signed(
                                false,
                                VerificationType::Deferred(verify_map_params.clone()),
                            )))
                        }
                        _ => {
                            log::debug!("Signature verification failed: {e}");
                            Err(SignedRejection::SignatureInvalid)
                        }
                    },
                }
            }
            _ => {
                log::debug!("Multiple signatures in header");
                Err(SignedRejection::MultipleSignatures)
            }
        }
    }
}

impl FromRequestParts<AppState> for Permitted {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let remote_addr: Option<String> = get_header(parts, "x-forwarded-for")
            .and_then(|s| s.split(',').next().map(|s| s.trim().to_string()));

        if let Some(ip) = remote_addr {
            if state.block_list.is_blocked(ip.to_string()) {
                log::warn!("Blocking request from IP: {ip}");
                return Ok(Permitted(false));
            }
        }
        Ok(Permitted(true))
    }
}
