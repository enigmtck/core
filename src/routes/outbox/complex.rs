use crate::routes::Outbox;
use jdt_maybe_multiple::MaybeMultiple;
use serde_json::Value;

impl Outbox for MaybeMultiple<Value> {}
