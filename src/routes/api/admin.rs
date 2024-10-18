use crate::{
    admin::{self, NewUser},
    db::Db,
    models::actors::Actor,
};
use rocket::{http::Status, post, serde::json::Error, serde::json::Json};

#[post("/api/user/create", format = "json", data = "<user>")]
pub async fn create_user(
    conn: Db,
    user: Result<Json<NewUser>, Error<'_>>,
) -> Result<Json<Actor>, Status> {
    if let Ok(Json(user)) = user {
        log::debug!("CREATING USER\n{user:#?}");

        if let Ok(profile) = admin::create_user(Some(&conn), user).await {
            Ok(Json(profile))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}
