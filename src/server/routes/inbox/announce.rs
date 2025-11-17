use super::Inbox;
use crate::{
    db::runner::DbRunner,
    models::activities::{create_activity, NewActivity},
    runner,
    server::AppState,
};
use jdt_activity_pub::{ApActivity, ApAddress, ApAnnounce};
use reqwest::StatusCode;
use serde_json::Value;

impl Inbox for ApAnnounce {
    async fn inbox<C: DbRunner>(
        &self,
        conn: &C,
        state: AppState,
        raw: Value,
    ) -> Result<StatusCode, StatusCode> {
        log::debug!("{:?}", self.clone());

        let mut activity = NewActivity::try_from((ApActivity::Announce(self.clone()), None))
            .map_err(|e| {
                log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        activity.raw = Some(raw.clone());

        if create_activity(conn, activity.clone()).await.is_ok() {
            let pool = state.db_pool.clone();
            let ap_id = activity.ap_id.clone().ok_or(StatusCode::BAD_REQUEST)?;

            runner::run(
                runner::announce::remote_announce_task,
                pool,
                None,
                vec![ap_id],
            )
            .await;

            Ok(StatusCode::ACCEPTED)
        } else {
            log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}
