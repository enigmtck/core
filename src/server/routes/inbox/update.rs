use super::Inbox;
use crate::{
    db::runner::DbRunner,
    models::{
        activities::{create_activity, NewActivity},
        actors::{create_or_update_actor, NewActor},
        objects::create_object,
    },
    GetWebfinger,
};
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{ApActivity, ApAddress, ApObject, ApUpdate};
use reqwest::StatusCode;
use serde_json::Value;

impl Inbox for ApUpdate {
    async fn inbox<C: DbRunner>(
        &self,
        conn: &C,
        _pool: Pool,
        raw: Value,
    ) -> Result<StatusCode, StatusCode> {
        log::debug!("{:?}", self.clone());

        let activity: ApActivity = self.clone().into();

        match self.clone().object {
            MaybeReference::Actual(actual) => match actual {
                ApObject::Actor(actor) => {
                    log::debug!("{actor}");
                    let webfinger = actor.get_webfinger().await;

                    if let Ok(mut new_remote_actor) = NewActor::try_from(actor.clone()) {
                        new_remote_actor.ek_webfinger = webfinger;

                        if actor.clone().id.unwrap_or_default() == self.actor.clone() {
                            let actor = create_or_update_actor(conn, new_remote_actor)
                                .await
                                .map_err(|e| {
                                    log::error!("Failed to create or update Actor: {e}");
                                    StatusCode::INTERNAL_SERVER_ERROR
                                })?;

                            let mut activity =
                                NewActivity::try_from((activity, Some(actor.into()))).map_err(
                                    |e| {
                                        log::error!("Failed to build NewActivity: {e}");
                                        StatusCode::INTERNAL_SERVER_ERROR
                                    },
                                )?;
                            activity.raw = Some(raw);

                            create_activity(conn, activity).await.map_err(|e| {
                                log::error!("Failed to create Activity: {e}");
                                StatusCode::INTERNAL_SERVER_ERROR
                            })?;

                            Ok(StatusCode::ACCEPTED)
                        } else {
                            log::error!("Failed to handle Activity");
                            Err(StatusCode::UNAUTHORIZED)
                        }
                    } else {
                        log::error!("Failed to handle Activity");
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
                ApObject::Note(note) => {
                    log::debug!("{note}");
                    if note.clone().attributed_to == self.actor.clone() {
                        let object = create_object(conn, note.into()).await.map_err(|e| {
                            log::error!("Failed to create or update Note: {e}");
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;

                        let mut activity = NewActivity::try_from((activity, Some(object.into())))
                            .map_err(|e| {
                            log::error!("Failed to build NewActivity: {e}");
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;
                        activity.raw = Some(raw);

                        create_activity(conn, activity).await.map_err(|e| {
                            log::error!("Failed to create Activity: {e}");
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;

                        Ok(StatusCode::ACCEPTED)
                    } else {
                        log::error!("attributed_to does not match Actor");
                        Err(StatusCode::UNAUTHORIZED)
                    }
                }
                ApObject::Article(article) => {
                    log::debug!("{article}");
                    if article.clone().attributed_to == self.actor.clone() {
                        let object = create_object(conn, article.into()).await.map_err(|e| {
                            log::error!("Failed to create or update Article: {e}");
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;

                        let mut activity = NewActivity::try_from((activity, Some(object.into())))
                            .map_err(|e| {
                            log::error!("Failed to build NewActivity: {e}");
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;
                        activity.raw = Some(raw);

                        create_activity(conn, activity).await.map_err(|e| {
                            log::error!("Failed to create Activity: {e}");
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;

                        Ok(StatusCode::ACCEPTED)
                    } else {
                        log::error!("attributed_to does not match Actor");
                        Err(StatusCode::UNAUTHORIZED)
                    }
                }
                ApObject::Question(question) => {
                    log::debug!("{question}");
                    if question.clone().attributed_to == self.actor.clone() {
                        let object = create_object(conn, question.into()).await.map_err(|e| {
                            log::error!("Failed to create or update Question: {e}");
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;

                        let mut activity = NewActivity::try_from((activity, Some(object.into())))
                            .map_err(|e| {
                            log::error!("Failed to build NewActivity: {e}");
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;
                        activity.raw = Some(raw);

                        create_activity(conn, activity).await.map_err(|e| {
                            log::error!("Failed to create Activity: {e}");
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?;

                        Ok(StatusCode::ACCEPTED)
                    } else {
                        log::error!("attributed_to does not match Actor");
                        Err(StatusCode::UNAUTHORIZED)
                    }
                }
                _ => {
                    log::debug!("Unimplemented Update type");
                    Err(StatusCode::NOT_IMPLEMENTED)
                }
            },
            _ => Err(StatusCode::UNPROCESSABLE_ENTITY),
        }
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}
