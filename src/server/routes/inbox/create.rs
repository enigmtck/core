use super::Inbox;

use crate::{
    db::runner::DbRunner,
    models::{
        activities::{create_activity, get_activity_by_ap_id, ActivityTarget, NewActivity},
        objects::{create_or_update_object, NewObject},
        unprocessable::create_unprocessable,
    },
    runner,
};
use anyhow::Result;
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{ApActivity, ApAddress, ApCreate, ApObject};
use reqwest::StatusCode;
use serde_json::Value;

impl Inbox for ApCreate {
    async fn inbox<C: DbRunner>(
        &self,
        conn: &C,
        pool: Pool,
        raw: Value,
    ) -> Result<StatusCode, StatusCode> {
        log::debug!("{:?}", self.clone());

        if let Some(id) = self.id.clone() {
            if get_activity_by_ap_id(conn, id).await.is_ok() {
                return Ok(StatusCode::ACCEPTED);
            }
        }

        match self.clone().object {
            MaybeReference::Actual(ApObject::Note(x)) => {
                let new_object = NewObject::from(x.clone());

                let object = create_or_update_object(conn, new_object)
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO CREATE OR UPDATE OBJECT: {e:#?}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                let mut activity = NewActivity::try_from((
                    ApActivity::Create(self.clone()),
                    Some(ActivityTarget::from(object.clone())),
                ))
                .map_err(|e| {
                    log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

                activity.raw = Some(raw);

                if create_activity(conn, activity).await.is_ok() {
                    let pool = pool.clone();
                    let object_id = object.as_id.clone();

                    tokio::spawn(async move {
                        if let Err(e) = runner::note::object_task(pool, None, vec![object_id]).await
                        {
                            log::error!("Failed to run object_task for note: {e:?}");
                        }
                    });

                    Ok(StatusCode::ACCEPTED)
                } else {
                    log::error!("FAILED TO INSERT ACTIVITY");
                    Err(StatusCode::NO_CONTENT)
                }
            }
            MaybeReference::Actual(ApObject::Article(article)) => {
                let new_object = NewObject::from(article.clone());

                let object = create_or_update_object(conn, new_object)
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO CREATE OR UPDATE ARTICLE: {e:#?}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                let mut activity = NewActivity::try_from((
                    ApActivity::Create(self.clone()),
                    Some(ActivityTarget::from(object.clone())),
                ))
                .map_err(|e| {
                    log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

                activity.raw = Some(raw);

                if create_activity(conn, activity).await.is_ok() {
                    let pool = pool.clone();
                    let object_id = object.as_id.clone();

                    tokio::spawn(async move {
                        if let Err(e) = runner::note::object_task(pool, None, vec![object_id]).await
                        {
                            log::error!("Failed to run object_task for article: {e:?}");
                        }
                    });
                    Ok(StatusCode::ACCEPTED)
                } else {
                    log::error!("FAILED TO INSERT ACTIVITY");
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
            MaybeReference::Actual(ApObject::Question(question)) => {
                let new_object = NewObject::from(question.clone());

                let object = create_or_update_object(conn, new_object)
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO CREATE OR UPDATE Object: {e:#?}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                let mut activity = NewActivity::try_from((
                    ApActivity::Create(self.clone()),
                    Some(ActivityTarget::from(object.clone())),
                ))
                .map_err(|e| {
                    log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

                activity.raw = Some(raw);

                if create_activity(conn, activity).await.is_ok() {
                    let pool = pool.clone();
                    let object_id = object.as_id.clone();

                    tokio::spawn(async move {
                        if let Err(e) = runner::note::object_task(pool, None, vec![object_id]).await
                        {
                            log::error!("Failed to run object_task for question: {e:?}");
                        }
                    });
                    Ok(StatusCode::ACCEPTED)
                } else {
                    log::error!("FAILED TO INSERT ACTIVITY");
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
            _ => {
                log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
                create_unprocessable(
                    conn,
                    (raw, Some("Create object not implemented".to_string())).into(),
                )
                .await;
                Err(StatusCode::NOT_IMPLEMENTED)
            }
        }
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}
