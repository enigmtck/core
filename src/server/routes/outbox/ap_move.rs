use crate::server::routes::Outbox;
use crate::{
    db::runner::DbRunner,
    models::{
        activities::{create_activity, NewActivity},
        actors::Actor,
    },
};
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::{ApActivity, ApMove};
use reqwest::StatusCode;
use serde_json::Value;

use super::ActivityJson;

impl Outbox for ApMove {
    async fn outbox<C: DbRunner>(
        &self,
        conn: &C,
        _pool: Pool,
        _profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        log::debug!("{:?}", self.clone());

        let mut activity =
            NewActivity::try_from((ApActivity::Move(self.clone()), None)).map_err(|e| {
                log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        activity.raw = Some(raw.clone());

        create_activity(conn, activity.clone()).await.map_err(|e| {
            log::error!("FAILED TO CREATE ACTIVITY: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        Ok(ActivityJson(ApActivity::Move(self.clone())))
    }
}
