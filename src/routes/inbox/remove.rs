use super::Inbox;
use crate::{
    db::Db,
    models::activities::{create_activity, NewActivity},
};
use jdt_activity_pub::{ApActivity, ApAddress, ApRemove};
use rocket::http::Status;
use serde_json::Value;

impl Inbox for ApRemove {
    async fn inbox(&self, conn: Db, raw: Value) -> Result<Status, Status> {
        log::debug!("{:?}", self.clone());

        let mut activity = NewActivity::try_from((ApActivity::Remove(self.clone()), None))
            .map_err(|e| {
                log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                Status::InternalServerError
            })?;
        activity.raw = Some(raw.clone());

        create_activity(&conn, activity.clone())
            .await
            .map_err(|e| {
                log::error!("FAILED TO CREATE ACTIVITY: {e:#?}");
                Status::InternalServerError
            })?;

        Ok(Status::Accepted)
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}
