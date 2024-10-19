use rocket::http::Status;
use rocket::{post, serde::json::Error, serde::json::Json};
use serde::Deserialize;

use crate::admin::{self, verify_and_generate_password};
use crate::db::Db;
use crate::fairings::signatures::Signed;
use crate::models::actors::Actor;
use crate::models::pg::actors::update_password_by_username;
use crate::models::profiles::Profile;

#[derive(Deserialize, Debug, Clone)]
pub struct AuthenticationData {
    pub username: String,
    pub password: String,
}

#[post("/api/user/authenticate", format = "json", data = "<user>")]
pub async fn authenticate_user(
    conn: Db,
    user: Result<Json<AuthenticationData>, Error<'_>>,
) -> Result<Json<Profile>, Status> {
    log::debug!("AUTHENTICATING\n{user:#?}");

    let user = user.map_err(|_| Status::Unauthorized)?;
    let profile = admin::authenticate(&conn, user.username.clone(), user.password.clone())
        .await
        .ok_or(Status::Unauthorized)?;

    Ok(Json(profile))
}

// We need to include the re-encrypted private data that is encrypted using a derivation of the
// plaintext password (the current and updated "passwords" submitted here are base64-encoded Blake2b
// hashes of the real passwords - the plaintext password is never processed by this server)
#[derive(Deserialize, Debug, Clone)]
pub struct UpdatePassword {
    pub current: String,
    pub updated: String,
    pub encrypted_client_private_key: String,
    pub encrypted_olm_pickled_account: String,
}

#[post("/api/user/<username>/password", format = "json", data = "<password>")]
pub async fn change_password(
    signed: Signed,
    conn: Db,
    username: String,
    password: Result<Json<UpdatePassword>, Error<'_>>,
) -> Result<Json<Actor>, Status> {
    if signed.local() {
        let password = password.map_err(|_| Status::Unauthorized)?;

        let client_private_key = password.encrypted_client_private_key.clone();
        let olm_pickled_account = password.encrypted_olm_pickled_account.clone();

        let password = verify_and_generate_password(
            &conn,
            username.clone(),
            password.current.clone(),
            password.updated.clone(),
        )
        .await
        .ok_or(Status::Unauthorized)?;

        Ok(Json(
            update_password_by_username(
                &conn,
                username,
                password,
                client_private_key,
                olm_pickled_account,
            )
            .await
            .unwrap_or_default(),
        ))
    } else {
        Err(Status::Unauthorized)
    }
}
