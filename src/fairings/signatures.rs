use std::collections::HashMap;

use crate::{
    db::Db,
    models::{
        actors::{get_actor_by_key_id, Actor},
        instances::{create_or_update_instance, Instance},
    },
    signing::{verify, VerificationError, VerificationType, VerifyMapParams},
    ASSIGNMENT_RE, DOMAIN_RE,
};
use anyhow::{anyhow, Result};
use jdt_activity_pub::ApActor;
use rocket::{
    http::Status,
    request::{FromRequest, Outcome, Request},
};

#[derive(Clone, Debug)]
pub struct Signed(pub bool, pub VerificationType);

impl Signed {
    pub fn local(&self) -> bool {
        matches!(self, Signed(true, VerificationType::Local(_)))
    }

    pub fn remote(&self) -> bool {
        matches!(self, Signed(true, VerificationType::Remote(_)))
    }

    pub fn any(&self) -> bool {
        matches!(self, Signed(true, _))
    }

    pub fn actor(&self) -> Option<ApActor> {
        if let Signed(true, VerificationType::Remote((actor, _digest))) = self {
            Some(*actor.clone())
        } else {
            None
        }
    }

    pub fn profile(&self) -> Option<Actor> {
        if let Signed(true, VerificationType::Local((profile, _digest))) = self {
            Some(*profile.clone())
        } else {
            None
        }
    }

    pub fn digest(&self) -> Option<String> {
        if let Signed(true, VerificationType::Local((_profile, digest))) = self {
            digest.clone()
        } else if let Signed(true, VerificationType::Remote((_actor, digest))) = self {
            digest.clone()
        } else {
            None
        }
    }

    pub fn deferred(&self) -> Option<VerifyMapParams> {
        if let Signed(false, VerificationType::Deferred(params)) = self {
            Some(*params.clone())
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum SignatureError {
    NoDateProvided,
    NoHostProvided,
    NoDbConnection,
    NonExistent,
    MultipleSignatures,
    InvalidRequestPath,
    InvalidRequestUsername,
    LocalUserNotFound,
    SignatureInvalid,
    Unknown,
}

async fn update_instance(conn: &Db, signature: String) -> Result<Instance> {
    let mut signature_map = HashMap::<String, String>::new();

    for cap in ASSIGNMENT_RE.captures_iter(&signature) {
        signature_map.insert(cap[1].to_string(), cap[2].to_string());
    }

    let key_id = signature_map
        .get("keyId")
        .ok_or(anyhow!("keyId not found in signature_map"))?;

    // It might be better to derive the domain_name from the webfinger stored
    // on the Actor. But I'm not sure how that would work when the actor doesn't
    // exist yet, so maybe not.
    let domain_name = DOMAIN_RE
        .captures(key_id)
        .ok_or(anyhow!("failed to retrieve key_id"))?[1]
        .to_string();

    let shared_inbox = get_actor_by_key_id(conn, key_id.clone())
        .await
        .ok()
        .and_then(|actor| {
            ApActor::from(actor)
                .endpoints
                .map(|endpoints| endpoints.shared_inbox)
        });

    create_or_update_instance(conn, (domain_name, shared_inbox).into()).await
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Signed {
    type Error = SignatureError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Retrieve a header value by name
        let get_header = |header_name| {
            request
                .headers()
                .get(header_name)
                .next()
                .map(|val| val.to_string())
        };

        match request.guard::<Db>().await {
            Outcome::Success(conn) => {
                let method = request.method().to_string();
                let host = (*crate::SERVER_NAME).clone();
                //let host = request.host().expect("Host not found").to_string();
                let path = request.uri().path().to_string();
                let path = path.trim_end_matches('&');
                let request_target = format!("{} {}", method.to_lowercase(), path);

                let date = match get_header("date").or_else(|| get_header("enigmatick-date")) {
                    Some(val) => val,
                    None => {
                        return Outcome::Success(Signed(false, VerificationType::None));
                    }
                };

                let digest = get_header("digest");
                let user_agent = get_header("user-agent");
                let content_length = get_header("content-length");
                let content_type = get_header("content-type");

                let signature_vec: Vec<_> = request.headers().get("signature").collect();

                match signature_vec.len() {
                    0 => Outcome::Success(Signed(false, VerificationType::None)),
                    1 => {
                        let signature = signature_vec[0].to_string();

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

                        match verify(&conn, verify_params.clone()).await {
                            Ok(t) => {
                                log::debug!("Signature verification successful");
                                let _ = update_instance(&conn, signature.to_string()).await;
                                Outcome::Success(Signed(true, t))
                            }
                            Err(e) => match e {
                                // It's possible that we've never seen the signing actor before and don't
                                // yet have them in the database. The ID of the user is not available in
                                // the Fairing (i.e., it's not in the header), so we defer the verification
                                // to the receiving route that decodes the whole request
                                VerificationError::ActorNotFound(ref verify_map_params) => {
                                    log::debug!("Signature verification deferred: {e}");
                                    Outcome::Success(Signed(
                                        false,
                                        VerificationType::Deferred(verify_map_params.clone()),
                                    ))
                                }
                                _ => {
                                    log::debug!("Signature verification failed: {e}");
                                    Outcome::Error((
                                        Status::BadRequest,
                                        SignatureError::SignatureInvalid,
                                    ))
                                }
                            },
                        }
                    }
                    _ => {
                        log::debug!("Multiple signatures in header");
                        Outcome::Error((Status::BadRequest, SignatureError::MultipleSignatures))
                    }
                }
            }
            Outcome::Error(e) => {
                log::error!("Unable to connect to database: {e:?}");
                Outcome::Error((Status::InternalServerError, SignatureError::NoDbConnection))
            }
            _ => {
                log::error!("Unknown error");
                Outcome::Error((Status::InternalServerError, SignatureError::Unknown))
            }
        }
    }
}
