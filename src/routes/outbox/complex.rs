use crate::routes::Outbox;
use crate::MaybeMultiple;
use serde_json::Value;

impl Outbox for MaybeMultiple<Value> {}
