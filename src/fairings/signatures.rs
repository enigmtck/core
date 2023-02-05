use crate::{
    db::{get_profile_by_username, Db},
    signing::{verify, VerificationType, VerifyParams},
};

use rocket::{
    http::Status,
    request::{FromRequest, Outcome, Request},
};

pub struct Signed(pub bool, pub VerificationType);

#[derive(Debug)]
pub enum SignatureError {
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
        let conn = request.guard::<Db>().await.unwrap();
        let method = request.method().to_string();
        let host = request.host().unwrap().to_string();
        //let path = request.uri().path().to_string();
        let path = request.uri().to_string();
        let path = path.trim_end_matches('&');

        //log::debug!("request: {:#?}", request);

        let username_re = regex::Regex::new(r"(?:/api)?(/user/)([a-zA-Z0-9_]+)(/.*)").unwrap();
        if let Some(username_match) = username_re.captures(path) {
            if let Some(username) = username_match.get(2) {
                match get_profile_by_username(&conn, username.as_str().to_string()).await {
                    Some(profile) => {
                        let request_target = format!("{} {}", method.to_lowercase(), path);

                        let mut date = String::new();
                        let date_vec: Vec<_> = request.headers().get("date").collect();
                        if date_vec.len() == 1 {
                            date = date_vec[0].to_string();
                        } else {
                            // browser fetch is a jerk and forbids the "date" header; browsers
                            // aggressively strip it, so I use Enigmatick-Date as a backup
                            let enigmatick_date_vec: Vec<_> =
                                request.headers().get("enigmatick-date").collect();

                            if enigmatick_date_vec.len() == 1 {
                                date = enigmatick_date_vec[0].to_string();
                            }
                        }

                        let mut digest = Option::<String>::None;
                        let digest_vec: Vec<_> = request.headers().get("digest").collect();
                        if digest_vec.len() == 1 {
                            digest = Option::from(digest_vec[0].to_string());
                        }

                        let content_type = request.content_type().unwrap().to_string();

                        let signature_vec: Vec<_> = request.headers().get("signature").collect();
                        //let signature = signature_vec[0].to_string();

                        match signature_vec.len() {
                            0 => {
                                Outcome::Failure((Status::BadRequest, SignatureError::NonExistent))
                            }
                            1 => {
                                let signature = signature_vec[0].to_string();

                                let (x, t) = verify(
                                    conn,
                                    VerifyParams {
                                        profile,
                                        signature,
                                        request_target,
                                        host,
                                        date,
                                        digest,
                                        content_type,
                                    },
                                )
                                .await;

                                Outcome::Success(Signed(x, t))
                            }
                            _ => Outcome::Failure((
                                Status::BadRequest,
                                SignatureError::MultipleSignatures,
                            )),
                        }
                    }
                    None => {
                        Outcome::Failure((Status::BadRequest, SignatureError::LocalUserNotFound))
                    }
                }
            } else {
                Outcome::Failure((Status::BadRequest, SignatureError::InvalidRequestUsername))
            }
        } else {
            Outcome::Failure((Status::BadRequest, SignatureError::InvalidRequestPath))
        }
    }
}
