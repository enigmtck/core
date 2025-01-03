use super::Inbox;
use crate::{
    db::Db,
    models::activities::{create_activity, NewActivity},
    runner,
};
use jdt_activity_pub::{ApActivity, ApAddress, ApAnnounce};
use rocket::http::Status;
use serde_json::Value;

impl Inbox for ApAnnounce {
    async fn inbox(&self, conn: Db, raw: Value) -> Result<Status, Status> {
        let mut activity = NewActivity::try_from((ApActivity::Announce(self.clone()), None))
            .map_err(|e| {
                log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                Status::InternalServerError
            })?;

        activity.raw = Some(raw.clone());

        if create_activity((&conn).into(), activity.clone())
            .await
            .is_ok()
        {
            runner::run(
                runner::announce::remote_announce_task,
                conn,
                None,
                vec![activity.ap_id.clone().ok_or(Status::BadRequest)?],
            )
            .await;
            Ok(Status::Accepted)
        } else {
            log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
            Err(Status::new(521))
        }
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}
