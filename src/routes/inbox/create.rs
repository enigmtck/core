use super::Inbox;

use crate::{
    db::Db,
    models::{
        activities::{create_activity, get_activity_by_ap_id, ActivityTarget, NewActivity},
        objects::{create_or_update_object, NewObject},
        unprocessable::create_unprocessable,
    },
    runner,
};
use anyhow::Result;
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{ApActivity, ApAddress, ApCreate, ApObject};
use rocket::http::Status;
use serde_json::Value;

impl Inbox for ApCreate {
    async fn inbox(&self, conn: Db, raw: Value) -> Result<Status, Status> {
        log::debug!("{:?}", self.clone());

        if let Some(id) = self.id.clone() {
            if get_activity_by_ap_id(&conn, id).await.is_ok() {
                return Ok(Status::Accepted);
            }
        }

        match self.clone().object {
            MaybeReference::Actual(ApObject::Note(x)) => {
                let new_object = NewObject::from(x.clone());

                let object = create_or_update_object(&conn, new_object)
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO CREATE OR UPDATE OBJECT: {e:#?}");
                        Status::InternalServerError
                    })?;

                let mut activity = NewActivity::try_from((
                    ApActivity::Create(self.clone()),
                    Some(ActivityTarget::from(object.clone())),
                ))
                .map_err(|e| {
                    log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                    Status::InternalServerError
                })?;

                activity.raw = Some(raw);

                if create_activity(&conn, activity).await.is_ok() {
                    runner::run(runner::note::object_task, conn, None, vec![object.as_id]).await;

                    Ok(Status::Accepted)
                } else {
                    log::error!("FAILED TO INSERT ACTIVITY");
                    Err(Status::NoContent)
                }
            }
            MaybeReference::Actual(ApObject::Article(article)) => {
                let new_object = NewObject::from(article.clone());

                let object = create_or_update_object(&conn, new_object)
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO CREATE OR UPDATE ARTICLE: {e:#?}");
                        Status::InternalServerError
                    })?;

                let mut activity = NewActivity::try_from((
                    ApActivity::Create(self.clone()),
                    Some(ActivityTarget::from(object.clone())),
                ))
                .map_err(|e| {
                    log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                    Status::InternalServerError
                })?;

                activity.raw = Some(raw);

                if create_activity(&conn, activity).await.is_ok() {
                    runner::run(runner::note::object_task, conn, None, vec![object.as_id]).await;
                    Ok(Status::Accepted)
                } else {
                    log::error!("FAILED TO INSERT ACTIVITY");
                    Err(Status::InternalServerError)
                }
            }
            MaybeReference::Actual(ApObject::Question(question)) => {
                let new_object = NewObject::from(question.clone());

                let object = create_or_update_object(&conn, new_object)
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO CREATE OR UPDATE Object: {e:#?}");
                        Status::InternalServerError
                    })?;

                let mut activity = NewActivity::try_from((
                    ApActivity::Create(self.clone()),
                    Some(ActivityTarget::from(object.clone())),
                ))
                .map_err(|e| {
                    log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                    Status::InternalServerError
                })?;

                activity.raw = Some(raw);

                if create_activity(&conn, activity).await.is_ok() {
                    runner::run(runner::note::object_task, conn, None, vec![object.as_id]).await;
                    Ok(Status::Accepted)
                } else {
                    log::error!("FAILED TO INSERT ACTIVITY");
                    Err(Status::InternalServerError)
                }
            }
            _ => {
                log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
                create_unprocessable(
                    &conn,
                    (raw, Some("Create object not implemented".to_string())).into(),
                )
                .await;
                Err(Status::NotImplemented)
            }
        }
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}
