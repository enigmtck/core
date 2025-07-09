use super::Inbox;
use crate::{
    db::runner::DbRunner,
    models::activities::{create_activity, NewActivity},
};
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::{ApActivity, ApAddress, ApRemove};
use reqwest::StatusCode;
use serde_json::Value;

impl Inbox for ApRemove {
    async fn inbox<C: DbRunner>(
        &self,
        conn: &C,
        _pool: Pool,
        raw: Value,
    ) -> Result<StatusCode, StatusCode> {
        log::debug!("{:?}", self.clone());

        let mut activity = NewActivity::try_from((ApActivity::Remove(self.clone()), None))
            .map_err(|e| {
                log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        activity.raw = Some(raw.clone());

        create_activity(conn, activity.clone()).await.map_err(|e| {
            log::error!("FAILED TO CREATE ACTIVITY: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        Ok(StatusCode::ACCEPTED)
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}
