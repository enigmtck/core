use super::Inbox;
use crate::{
    db::runner::DbRunner,
    models::{
        activities::{create_activity, ActivityTarget, NewActivity},
        objects::get_object_by_as_id,
    },
    server::AppState,
};
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{ApActivity, ApAddress, ApLike, ApObject};
use reqwest::StatusCode;
use serde_json::Value;

impl Inbox for Box<ApLike> {
    async fn inbox<C: DbRunner>(
        &self,
        conn: &C,
        _state: AppState,
        raw: Value,
    ) -> Result<StatusCode, StatusCode> {
        log::debug!("{:?}", self.clone());

        let note_apid = match self.object.clone() {
            MaybeReference::Reference(reference) => Some(reference),
            MaybeReference::Actual(ApObject::Note(actual)) => actual.id,
            _ => None,
        };

        let note_apid = note_apid.ok_or(StatusCode::BAD_REQUEST)?;

        let target = get_object_by_as_id(conn, note_apid).await.map_err(|e| {
            log::debug!("LIKE TARGET NOT FOUND: {e:#?}");
            StatusCode::NOT_FOUND
        })?;

        let mut activity = NewActivity::try_from((
            ApActivity::Like(self.clone()),
            Some(ActivityTarget::from(target)),
        ))
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
