use crate::db::Db;
use crate::models::actors::Actor;
use enum_dispatch::enum_dispatch;
use jdt_activity_pub::{
    ApAccept, ApActivity, ApActor, ApAdd, ApAddress, ApAnnounce, ApBasicContent, ApBlock,
    ApCollection, ApCreate, ApDelete, ApFollow, ApInstrument, ApLike, ApNote, ApObject, ApQuestion,
    ApSession, ApTombstone, ApUndo, ApUpdate,
};
use jdt_maybe_multiple::MaybeMultiple;
use jdt_maybe_reference::Identifier;
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

impl Inbox for ApActivity {
    async fn inbox(&self, conn: Db, raw: Value) -> Result<Status, Status> {
        match self {
            ApActivity::Delete(delete) => (*delete).inbox(conn, raw).await,
            ApActivity::Like(like) => (*like).inbox(conn, raw).await,
            ApActivity::Undo(undo) => (*undo).inbox(conn, raw).await,
            ApActivity::Accept(accept) => (*accept).inbox(conn, raw).await,
            ApActivity::Follow(follow) => follow.inbox(conn, raw).await,
            ApActivity::Announce(announce) => announce.inbox(conn, raw).await,
            ApActivity::Create(create) => create.inbox(conn, raw).await,
            ApActivity::Update(update) => update.inbox(conn, raw).await,
            ApActivity::Block(block) => block.inbox(conn, raw).await,
            ApActivity::Add(add) => add.inbox(conn, raw).await,
        }
    }

    fn actor(&self) -> ApAddress {
        match self {
            ApActivity::Delete(delete) => delete.actor.clone(),
            ApActivity::Like(like) => like.actor.clone(),
            ApActivity::Undo(undo) => undo.actor.clone(),
            ApActivity::Accept(accept) => accept.actor.clone(),
            ApActivity::Follow(follow) => follow.actor.clone(),
            ApActivity::Announce(announce) => announce.actor.clone(),
            ApActivity::Create(create) => create.actor.clone(),
            ApActivity::Update(update) => update.actor.clone(),
            ApActivity::Block(block) => block.actor.clone(),
            ApActivity::Add(add) => add.actor.clone(),
        }
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

impl Outbox for ApActivity {
    async fn outbox(
        &self,
        conn: Db,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        match self {
            ApActivity::Delete(delete) => (*delete).outbox(conn, profile, raw).await,
            ApActivity::Like(like) => (*like).outbox(conn, profile, raw).await,
            ApActivity::Undo(undo) => (*undo).outbox(conn, profile, raw).await,
            ApActivity::Accept(accept) => (*accept).outbox(conn, profile, raw).await,
            ApActivity::Follow(follow) => follow.outbox(conn, profile, raw).await,
            ApActivity::Announce(announce) => announce.outbox(conn, profile, raw).await,
            ApActivity::Create(create) => create.outbox(conn, profile, raw).await,
            ApActivity::Update(update) => update.outbox(conn, profile, raw).await,
            ApActivity::Block(block) => block.outbox(conn, profile, raw).await,
            ApActivity::Add(add) => add.outbox(conn, profile, raw).await,
        }
    }
}

impl Outbox for ApObject {
    async fn outbox(
        &self,
        conn: Db,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        match self {
            ApObject::Tombstone(x) => x.outbox(conn, profile, raw).await,
            ApObject::Session(x) => x.outbox(conn, profile, raw).await,
            ApObject::Instrument(x) => x.outbox(conn, profile, raw).await,
            ApObject::Note(x) => x.outbox(conn, profile, raw).await,
            ApObject::Question(x) => x.outbox(conn, profile, raw).await,
            ApObject::Actor(x) => x.outbox(conn, profile, raw).await,
            ApObject::Collection(x) => x.outbox(conn, profile, raw).await,
            ApObject::Identifier(x) => x.outbox(conn, profile, raw).await,
            ApObject::Basic(x) => x.outbox(conn, profile, raw).await,
            ApObject::Complex(x) => x.outbox(conn, profile, raw).await,
            ApObject::Plain(x) => x.outbox(conn, profile, raw).await,
        }
    }
}
