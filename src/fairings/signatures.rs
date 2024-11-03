use std::collections::HashMap;

use crate::{
    db::Db,
    models::{actors::Actor, instances::create_or_update_instance},
    signing::{verify, VerificationError, VerificationType, VerifyMapParams},
    ASSIGNMENT_RE, DOMAIN_RE,
};

use rocket::{
    http::Status,
    request::{FromRequest, Outcome, Request},
};

#[derive(Clone)]
pub struct Signed(pub bool, pub VerificationType);

impl Signed {
    pub fn local(&self) -> bool {
        matches!(self, Signed(true, VerificationType::Local(_)))
    }

    pub fn remote(&self) -> bool {
        matches!(self, Signed(true, VerificationType::Remote))
    }

    pub fn any(&self) -> bool {
        matches!(self, Signed(true, _))
    }

    pub fn profile(&self) -> Option<Actor> {
        if let Signed(true, VerificationType::Local(profile)) = self {
            Some(*profile.clone())
        } else {
            None
        }
    }

    pub fn deferred(&self) -> Option<VerifyMapParams> {
        if let Signed(true, VerificationType::Deferred(params)) = self {
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
}

async fn update_instance(conn: &Db, signature: String) {
    let mut signature_map = HashMap::<String, String>::new();

    for cap in ASSIGNMENT_RE.captures_iter(&signature) {
        signature_map.insert(cap[1].to_string(), cap[2].to_string());
    }

    let key_id = signature_map
        .get("keyId")
        .expect("keyId not found in signature_map");

    let domain_name = DOMAIN_RE
        .captures(key_id)
        .expect("unable to locate domain name")[1]
        .to_string();

    if let Err(e) = create_or_update_instance(Some(conn), domain_name.into()).await {
        log::error!("INSTANCE UPDATE ERROR: {e}");
    }
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
                let host = request.host().expect("Host not found").to_string();
                let path = request.uri().to_string();
                let path = path.trim_end_matches('&');
                let request_target = format!("{} {}", method.to_lowercase(), path);

                let date = match get_header("date").or_else(|| get_header("enigmatick-date")) {
                    Some(val) => val,
                    None => {
                        log::debug!("NO DATE PROVIDED");
                        return Outcome::Success(Signed(false, VerificationType::None));
                    }
                };

                let digest = get_header("digest");
                let user_agent = get_header("user-agent");
                let content_length = get_header("content-length");

                if let Some(content_type) = request.content_type() {
                    let content_type = content_type.to_string();
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
                            match verify(&conn, verify_params.clone()).await {
                                Ok(t) => {
                                    update_instance(&conn, signature.to_string()).await;
                                    Outcome::Success(Signed(true, t))
                                }
                                Err(e) => match e {
                                    // It's possible that we've never seen the signing actor before and don't
                                    // yet have them in the database. The ID of the user is not available in
                                    // the Fairing (i.e., it's not in the header), so we defer the verification
                                    // to the receiving route that decodes the whole request
                                    VerificationError::ActorNotFound(verify_map_params) => {
                                        Outcome::Success(Signed(
                                            false,
                                            VerificationType::Deferred(verify_map_params),
                                        ))
                                    }
                                    _ => {
                                        log::debug!("SIGNATURE VERIFICATION FAILED\n{e:#?}");
                                        Outcome::Error((
                                            Status::BadRequest,
                                            SignatureError::SignatureInvalid,
                                        ))
                                    }
                                },
                            }
                        }
                        _ => {
                            log::debug!("MULTIPLE SIGNATURES");
                            Outcome::Error((Status::BadRequest, SignatureError::MultipleSignatures))
                        }
                    }
                } else {
                    log::debug!("NO CONTENT-TYPE SPECIFIED");
                    Outcome::Success(Signed(false, VerificationType::None))
                }
            }
            Outcome::Error(e) => {
                log::error!("UNABLE TO CONNECT TO DATABASE: {e:?}");
                Outcome::Error((Status::InternalServerError, SignatureError::NoDbConnection))
            }
            _ => {
                log::error!("UNKNOWN PROBLEM");
                Outcome::Error((Status::InternalServerError, SignatureError::NoDbConnection))
            }
        }
    }
}
