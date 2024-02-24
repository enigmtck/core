use crate::{
    admin::{self, NewUser},
    db::Db,
    db::FlexibleDb,
    models::profiles::Profile,
};
use rocket::{http::Status, post, serde::json::Error, serde::json::Json};

#[post("/api/user/create", format = "json", data = "<user>")]
pub async fn create_user(
    conn: Db,
    user: Result<Json<NewUser>, Error<'_>>,
) -> Result<Json<Profile>, Status> {
    if let Ok(Json(user)) = user {
        log::debug!("CREATING USER\n{user:#?}");

        if let Some(profile) = admin::create_user(FlexibleDb::Db(&conn), user).await {
            Ok(Json(profile))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}
