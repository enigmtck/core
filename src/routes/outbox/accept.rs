use crate::{
    activity_pub::{ApAccept, ApActivity, Outbox},
    db::Db,
    fairings::events::EventChannels,
    models::actors::Actor,
    routes::ActivityJson,
};
use rocket::http::Status;
use serde_json::Value;

impl Outbox for Box<ApAccept> {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Actor,
        _raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        Err(Status::ServiceUnavailable)
    }
}
