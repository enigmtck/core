use crate::routes::Outbox;

// Activities
pub mod accept;
pub mod add;
pub mod announce;
pub mod ap_move;
pub mod block;
pub mod create;
pub mod delete;
pub mod follow;
pub mod like;
pub mod remove;
pub mod undo;
pub mod update;

// Objects
pub mod actor;
pub mod article;
pub mod collection;
pub mod complex;
pub mod identifier;
pub mod instrument;
pub mod note;
pub mod plain;
pub mod question;
pub mod session;
pub mod tombstone;
pub mod uncategorized;
