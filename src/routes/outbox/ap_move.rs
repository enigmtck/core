use super::Outbox;
use crate::{
    db::Db,
    models::{
        activities::{create_activity, NewActivity},
        actors::Actor,
    },
    routes::ActivityJson,
};
use jdt_activity_pub::{ApActivity, ApMove};
use rocket::http::Status;
use serde_json::Value;

impl Outbox for ApMove {
    async fn outbox(
        &self,
        conn: Db,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        log::debug!("{:?}", self.clone());

        let mut activity =
            NewActivity::try_from((ApActivity::Move(self.clone()), None)).map_err(|e| {
                log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                Status::InternalServerError
            })?;
        activity.raw = Some(raw.clone());

        create_activity((&conn).into(), activity.clone())
            .await
            .map_err(|e| {
                log::error!("FAILED TO CREATE ACTIVITY: {e:#?}");
                Status::InternalServerError
            })?;

        Ok(ActivityJson::from(ApActivity::Move(self.clone())))
    }
}
