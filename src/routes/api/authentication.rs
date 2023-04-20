use crate::admin::{self, verify_and_generate_password};
use crate::db::{update_password_by_username, Db};
use crate::fairings::signatures::Signed;
use crate::models::profiles::Profile;
use crate::signing::VerificationType;
use rocket::http::Status;
use rocket::{post, serde::json::Error, serde::json::Json};
use serde::Deserialize;

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

    if let Ok(user) = user {
        if let Some(profile) =
            admin::authenticate(&conn, user.username.clone(), user.password.clone()).await
        {
            Ok(Json(profile))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct UpdatePassword {
    pub current: String,
    pub updated: String,
}

#[post("/api/user/<username>/password", format = "json", data = "<password>")]
pub async fn change_password(
    signed: Signed,
    conn: Db,
    username: String,
    password: Result<Json<UpdatePassword>, Error<'_>>,
) -> Result<Json<Profile>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Ok(password) = password {
            if let Some(password) = verify_and_generate_password(
                &conn,
                username.clone(),
                password.current.clone(),
                password.updated.clone(),
            )
            .await
            {
                Ok(Json(
                    update_password_by_username(&conn, username, password)
                        .await
                        .unwrap_or_default(),
                ))
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::Forbidden)
        }
    } else {
        Err(Status::BadRequest)
    }
}
