use super::Inbox;
use crate::{
    db::Db,
    models::{
        activities::{create_activity, ActivityTarget, NewActivity},
        objects::get_object_by_as_id,
    },
};
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{ApActivity, ApAddress, ApLike, ApObject};
use rocket::http::Status;
use serde_json::Value;

impl Inbox for Box<ApLike> {
    async fn inbox(&self, conn: Db, raw: Value) -> Result<Status, Status> {
        log::debug!("{:?}", self.clone());

        let note_apid = match self.object.clone() {
            MaybeReference::Reference(reference) => Some(reference),
            MaybeReference::Actual(ApObject::Note(actual)) => actual.id,
            _ => None,
        };

        let note_apid = note_apid.ok_or(Status::BadRequest)?;

        let target = get_object_by_as_id(Some(&conn), note_apid)
            .await
            .map_err(|e| {
                log::debug!("LIKE TARGET NOT FOUND: {e:#?}");
                Status::NotFound
            })?;

        let mut activity = NewActivity::try_from((
            ApActivity::Like(self.clone()),
            Some(ActivityTarget::from(target)),
        ))
        .map_err(|e| {
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

        Ok(Status::Accepted)
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}
