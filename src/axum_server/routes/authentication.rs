use crate::{
    admin,
    axum_server::{extractors::AxumSigned, AppState},
    models::{
        actors::{update_password_by_username, Actor},
        profiles::Profile,
    },
    routes::{
        api::authentication::{AuthenticationData, UpdatePassword},
        ActivityJson,
    },
};
use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    response::Json,
};

pub async fn authenticate_user(
    State(state): State<AppState>,
    user: Result<Json<AuthenticationData>, JsonRejection>,
) -> Result<ActivityJson<Profile>, StatusCode> {
    log::debug!("AXUM AUTHENTICATING");

    let user = match user {
        Ok(json) => json.0,
        Err(e) => {
            log::error!("FAILED TO DECODE user: {e:#?}");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    let conn = match state.db_pool.get().await {
        Ok(conn) => conn,
        Err(e) => {
            log::error!("Failed to get DB connection: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    match admin::authenticate(&conn, user.username.clone(), user.password.clone()).await {
        Some(profile) => Ok(ActivityJson(profile)),
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn change_password(
    signed: AxumSigned,
    State(state): State<AppState>,
    Path(username): Path<String>,
    password: Result<Json<UpdatePassword>, JsonRejection>,
) -> Result<ActivityJson<Actor>, StatusCode> {
    if signed.local() {
        let password = match password {
            Ok(json) => json.0,
            Err(e) => {
                log::error!("FAILED TO DECODE password: {e:#?}");
                return Err(StatusCode::UNAUTHORIZED);
            }
        };

        let conn = match state.db_pool.get().await {
            Ok(conn) => conn,
            Err(e) => {
                log::error!("Failed to get DB connection: {e}");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

        let password_hash = match admin::verify_and_generate_password(
            &conn,
            username.clone(),
            password.current.clone(),
            password.updated.clone(),
        )
        .await
        {
            Some(hash) => hash,
            None => return Err(StatusCode::UNAUTHORIZED),
        };

        let client_private_key = password.encrypted_client_private_key.clone();
        let olm_pickled_account = password.encrypted_olm_pickled_account.clone();
        let olm_pickled_account_hash = password.olm_pickled_account_hash.clone();

        let updated_actor = update_password_by_username(
            &conn,
            username,
            password_hash,
            client_private_key,
            olm_pickled_account,
            olm_pickled_account_hash,
        )
        .await
        .unwrap_or_default();

        Ok(ActivityJson(updated_actor))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
