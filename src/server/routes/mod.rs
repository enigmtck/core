use crate::db::runner::DbRunner;
use crate::models::actors::Actor;
use crate::server::AppState;
use axum::response::Redirect;
use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
};
use enum_dispatch::enum_dispatch;
use jdt_activity_pub::{ApActivity, ApAddress, ApObject};
use serde::Serialize;
use serde_json::Value;

pub mod admin;
pub mod authentication;
pub mod client;
pub mod encryption;
pub mod image;
pub mod inbox;
pub mod instance;
pub mod objects;
pub mod outbox;
pub mod remote;
pub mod search;
pub mod user;
pub mod vault;
pub mod webfinger;

pub enum AbstractResponse<T> {
    ActivityJson(ActivityJson<T>),
    LdJson(LdJson<T>),
    JrdJson(JrdJson<T>),
    XrdXml(XrdXml),
    Redirect(Redirect),
    Json(Json<T>),
}

pub struct ActivityJson<T>(pub T);

impl<T: Serialize> IntoResponse for ActivityJson<T> {
    fn into_response(self) -> Response {
        let mut res = axum::response::Json(self.0).into_response();
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            "application/activity+json".parse().unwrap(),
        );
        res
    }
}

pub struct LdJson<T>(pub T);

impl<T: Serialize> IntoResponse for LdJson<T> {
    fn into_response(self) -> Response {
        let mut res = axum::response::Json(self.0).into_response();
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            "application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\""
                .parse()
                .unwrap(),
        );
        res
    }
}

pub struct JrdJson<T>(pub T);

impl<T: Serialize> IntoResponse for JrdJson<T> {
    fn into_response(self) -> Response {
        let mut res = axum::response::Json(self.0).into_response();
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            "application/jrd+json".parse().unwrap(),
        );
        res
    }
}

pub struct XrdXml(String);

impl IntoResponse for XrdXml {
    fn into_response(self) -> Response {
        let mut res = self.0.into_response();
        res.headers_mut()
            .insert(header::CONTENT_TYPE, "application/xrd+xml".parse().unwrap());
        res
    }
}

impl<T: Serialize> IntoResponse for AbstractResponse<T> {
    fn into_response(self) -> Response {
        match self {
            AbstractResponse::ActivityJson(json) => json.into_response(),
            AbstractResponse::LdJson(json) => json.into_response(),
            AbstractResponse::JrdJson(json) => json.into_response(),
            AbstractResponse::XrdXml(xml) => xml.into_response(),
            AbstractResponse::Redirect(redirect) => redirect.into_response(),
            AbstractResponse::Json(json) => json.into_response(),
        }
    }
}

#[enum_dispatch(ApActivity)]
pub trait Inbox {
    async fn inbox<C: DbRunner>(
        &self,
        _conn: &C,
        _state: AppState,
        _raw: Value,
    ) -> Result<StatusCode, StatusCode> {
        Err(StatusCode::NOT_IMPLEMENTED)
    }

    #[allow(dead_code)]
    fn actor(&self) -> ApAddress {
        ApAddress::None
    }
}

impl Inbox for ApActivity {
    async fn inbox<C: DbRunner>(
        &self,
        conn: &C,
        state: AppState,
        raw: Value,
    ) -> Result<StatusCode, StatusCode> {
        match self {
            ApActivity::Delete(delete) => (*delete).inbox(conn, state, raw).await,
            ApActivity::Like(like) => (*like).inbox(conn, state, raw).await,
            ApActivity::Undo(undo) => (*undo).inbox(conn, state, raw).await,
            ApActivity::Accept(accept) => (*accept).inbox(conn, state, raw).await,
            ApActivity::Follow(follow) => follow.inbox(conn, state, raw).await,
            ApActivity::Announce(announce) => announce.inbox(conn, state, raw).await,
            ApActivity::Create(create) => create.inbox(conn, state, raw).await,
            ApActivity::Update(update) => update.inbox(conn, state, raw).await,
            ApActivity::Block(block) => block.inbox(conn, state, raw).await,
            ApActivity::Add(add) => add.inbox(conn, state, raw).await,
            ApActivity::Remove(remove) => remove.inbox(conn, state, raw).await,
            ApActivity::Move(move_activity) => move_activity.inbox(conn, state, raw).await,
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
            ApActivity::Remove(remove) => remove.actor.clone(),
            ApActivity::Move(move_activity) => move_activity.actor.clone(),
        }
    }
}

#[enum_dispatch(ApActivity, ApObject)]
pub trait Outbox {
    async fn outbox<C: DbRunner>(
        &self,
        _conn: &C,
        _state: AppState,
        _profile: Actor,
        _raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        Err(StatusCode::NOT_IMPLEMENTED)
    }
}

impl Outbox for ApActivity {
    async fn outbox<C: DbRunner>(
        &self,
        conn: &C,
        state: AppState,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        match self {
            ApActivity::Delete(delete) => (*delete).outbox(conn, state, profile, raw).await,
            ApActivity::Like(like) => (*like).outbox(conn, state, profile, raw).await,
            ApActivity::Undo(undo) => (*undo).outbox(conn, state, profile, raw).await,
            ApActivity::Accept(accept) => (*accept).outbox(conn, state, profile, raw).await,
            ApActivity::Follow(follow) => follow.outbox(conn, state, profile, raw).await,
            ApActivity::Announce(announce) => announce.outbox(conn, state, profile, raw).await,
            ApActivity::Create(create) => create.outbox(conn, state, profile, raw).await,
            ApActivity::Update(update) => update.outbox(conn, state, profile, raw).await,
            ApActivity::Block(block) => block.outbox(conn, state, profile, raw).await,
            ApActivity::Add(add) => add.outbox(conn, state, profile, raw).await,
            ApActivity::Remove(remove) => remove.outbox(conn, state, profile, raw).await,
            ApActivity::Move(move_activity) => move_activity.outbox(conn, state, profile, raw).await,
        }
    }
}

impl Outbox for ApObject {
    async fn outbox<C: DbRunner>(
        &self,
        conn: &C,
        state: AppState,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        match self {
            ApObject::Tombstone(x) => x.outbox(conn, state, profile, raw).await,
            ApObject::Session(x) => x.outbox(conn, state, profile, raw).await,
            ApObject::Instrument(x) => x.outbox(conn, state, profile, raw).await,
            ApObject::Note(x) => x.outbox(conn, state, profile, raw).await,
            ApObject::Article(x) => x.outbox(conn, state, profile, raw).await,
            ApObject::Question(x) => x.outbox(conn, state, profile, raw).await,
            ApObject::Actor(x) => x.outbox(conn, state, profile, raw).await,
            ApObject::Collection(x) => x.outbox(conn, state, profile, raw).await,
            ApObject::Identifier(x) => x.outbox(conn, state, profile, raw).await,
        }
    }
}
