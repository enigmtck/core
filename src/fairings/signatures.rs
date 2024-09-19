use crate::{
    db::Db,
    models::profiles::Profile,
    signing::{verify, VerificationType, VerifyParams},
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

    pub fn profile(&self) -> Option<Profile> {
        match self {
            Signed(true, VerificationType::Local(profile)) => Some(*profile.clone()),
            _ => None,
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
                            let verify_params = VerifyParams {
                                signature,
                                request_target,
                                host,
                                date,
                                digest,
                                content_type,
                                content_length,
                                user_agent,
                            };
                            match verify(conn, verify_params.clone()).await {
                                Ok(t) => Outcome::Success(Signed(true, t)),
                                Err(e) => {
                                    log::debug!("{e:#?}");
                                    Outcome::Error((
                                        Status::BadRequest,
                                        SignatureError::SignatureInvalid,
                                    ))
                                }
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
