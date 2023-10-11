use crate::{
    db::Db,
    signing::{verify, VerificationType, VerifyParams},
};

use rocket::{
    http::Status,
    request::{FromRequest, Outcome, Request},
};

pub struct Signed(pub bool, pub VerificationType);

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

        let conn = request.guard::<Db>().await.expect("DB connection failed");

        let method = request.method().to_string();
        let host = request.host().expect("Host not found").to_string();
        let path = request.uri().to_string();
        let path = path.trim_end_matches('&');
        let request_target = format!("{} {}", method.to_lowercase(), path);

        let date = match get_header("date").or_else(|| get_header("enigmatick-date")) {
            Some(val) => val,
            None => return Outcome::Failure((Status::BadRequest, SignatureError::NoDateProvided)),
        };

        let digest = get_header("digest");
        let user_agent = get_header("user-agent");

        if let Some(content_type) = request.content_type() {
            let content_type = content_type.to_string();
            let signature_vec: Vec<_> = request.headers().get("signature").collect();

            match signature_vec.len() {
                0 => Outcome::Success(Signed(false, VerificationType::None)),
                1 => {
                    let signature = signature_vec[0].to_string();

                    match verify(
                        conn,
                        VerifyParams {
                            signature,
                            request_target,
                            host,
                            date,
                            digest,
                            content_type,
                            user_agent,
                        },
                    )
                    .await
                    {
                        Ok(t) => Outcome::Success(Signed(true, t)),
                        Err(_) => {
                            Outcome::Failure((Status::BadRequest, SignatureError::SignatureInvalid))
                        }
                    }
                }
                _ => Outcome::Failure((Status::BadRequest, SignatureError::MultipleSignatures)),
            }
        } else {
            Outcome::Success(Signed(false, VerificationType::None))
        }
    }
}
