use rocket::serde::json::Json;

use crate::activity_pub::ApActivity;

pub mod api;
pub mod inbox;
pub mod instance;
pub mod objects;
pub mod outbox;
pub mod retrieve;
pub mod user;
pub mod webfinger;

#[derive(Responder)]
#[response(content_type = "application/activity+json")]
pub struct ActivityJson<T>(Json<T>);

impl From<ApActivity> for ActivityJson<ApActivity> {
    fn from(activity: ApActivity) -> Self {
        ActivityJson(Json(activity))
    }
}

#[derive(Responder)]
#[response(content_type = "application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\"")]
pub struct LdJson<T>(Json<T>);

#[derive(Responder)]
#[response(content_type = "application/jrd+json")]
pub struct JrdJson<T>(Json<T>);

#[derive(Responder)]
#[response(content_type = "application/xrd+xml")]
pub struct XrdXml(String);
