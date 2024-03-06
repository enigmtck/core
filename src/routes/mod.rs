use rocket::serde::json::Json;

pub mod api;
pub mod inbox;
pub mod instance;
pub mod notes;
pub mod outbox;
pub mod retrieve;
pub mod user;
pub mod webfinger;

#[derive(Responder)]
#[response(content_type = "application/activity+json")]
pub struct ActivityJson<T>(Json<T>);
