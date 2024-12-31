use crate::activity_pub::{
    ApAccept, ApActivity, ApActor, ApAdd, ApAddress, ApAnnounce, ApBasicContent, ApBlock,
    ApCollection, ApCreate, ApDelete, ApFollow, ApInstrument, ApLike, ApNote, ApObject, ApQuestion,
    ApSession, ApTombstone, ApUndo, ApUpdate,
};
use crate::db::Db;
use crate::models::actors::Actor;
use crate::{Identifier, MaybeMultiple};
use enum_dispatch::enum_dispatch;
use rocket::http::Status;
use rocket::serde::json::Json;
use serde_json::Value;

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

#[enum_dispatch(ApActivity)]
pub trait Inbox {
    async fn inbox(&self, _conn: Db, _raw: Value) -> Result<Status, Status> {
        Err(Status::NotImplemented)
    }

    fn actor(&self) -> ApAddress {
        ApAddress::None
    }
}

#[enum_dispatch(ApActivity, ApObject)]
pub trait Outbox {
    async fn outbox(
        &self,
        _conn: Db,
        _profile: Actor,
        _raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        Err(Status::NotImplemented)
    }
}
