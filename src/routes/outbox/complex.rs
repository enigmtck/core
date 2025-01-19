use crate::routes::Outbox;
use jdt_activity_pub::MaybeMultiple;
use serde_json::Value;

impl Outbox for MaybeMultiple<Value> {}
