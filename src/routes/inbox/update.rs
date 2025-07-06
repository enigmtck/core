use super::Inbox;
use crate::{
    db::Db,
    models::{
        activities::{create_activity, NewActivity},
        actors::{create_or_update_actor, NewActor},
        objects::create_or_update_object,
    },
    GetWebfinger,
};
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{ApActivity, ApAddress, ApObject, ApUpdate};
use rocket::http::Status;
use serde_json::Value;

impl Inbox for ApUpdate {
    async fn inbox(&self, conn: Db, raw: Value) -> Result<Status, Status> {
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
                            let actor = create_or_update_actor(&conn, new_remote_actor)
                                .await
                                .map_err(|e| {
                                    log::error!("Failed to create or update Actor: {e}");
                                    Status::InternalServerError
                                })?;

                            let mut activity =
                                NewActivity::try_from((activity, Some(actor.into()))).map_err(
                                    |e| {
                                        log::error!("Failed to build NewActivity: {e}");
                                        Status::InternalServerError
                                    },
                                )?;
                            activity.raw = Some(raw);

                            create_activity(&conn, activity).await.map_err(|e| {
                                log::error!("Failed to create Activity: {e}");
                                Status::InternalServerError
                            })?;

                            Ok(Status::Accepted)
                        } else {
                            log::error!("Failed to handle Activity");
                            Err(Status::Unauthorized)
                        }
                    } else {
                        log::error!("Failed to handle Activity");
                        Err(Status::InternalServerError)
                    }
                }
                ApObject::Note(note) => {
                    log::debug!("{note}");
                    if note.clone().attributed_to == self.actor.clone() {
                        let object =
                            create_or_update_object(&conn, note.into())
                                .await
                                .map_err(|e| {
                                    log::error!("Failed to create or update Note: {e}");
                                    Status::InternalServerError
                                })?;

                        let mut activity = NewActivity::try_from((activity, Some(object.into())))
                            .map_err(|e| {
                            log::error!("Failed to build NewActivity: {e}");
                            Status::InternalServerError
                        })?;
                        activity.raw = Some(raw);

                        create_activity(&conn, activity).await.map_err(|e| {
                            log::error!("Failed to create Activity: {e}");
                            Status::InternalServerError
                        })?;

                        Ok(Status::Accepted)
                    } else {
                        log::error!("attributed_to does not match Actor");
                        Err(Status::Unauthorized)
                    }
                }
                ApObject::Article(article) => {
                    log::debug!("{article}");
                    if article.clone().attributed_to == self.actor.clone() {
                        let object = create_or_update_object(&conn, article.into())
                            .await
                            .map_err(|e| {
                                log::error!("Failed to create or update Article: {e}");
                                Status::InternalServerError
                            })?;

                        let mut activity = NewActivity::try_from((activity, Some(object.into())))
                            .map_err(|e| {
                            log::error!("Failed to build NewActivity: {e}");
                            Status::InternalServerError
                        })?;
                        activity.raw = Some(raw);

                        create_activity(&conn, activity).await.map_err(|e| {
                            log::error!("Failed to create Activity: {e}");
                            Status::InternalServerError
                        })?;

                        Ok(Status::Accepted)
                    } else {
                        log::error!("attributed_to does not match Actor");
                        Err(Status::Unauthorized)
                    }
                }
                ApObject::Question(question) => {
                    log::debug!("{question}");
                    if question.clone().attributed_to == self.actor.clone() {
                        let object = create_or_update_object(&conn, question.into())
                            .await
                            .map_err(|e| {
                                log::error!("Failed to create or update Question: {e}");
                                Status::InternalServerError
                            })?;

                        let mut activity = NewActivity::try_from((activity, Some(object.into())))
                            .map_err(|e| {
                            log::error!("Failed to build NewActivity: {e}");
                            Status::InternalServerError
                        })?;
                        activity.raw = Some(raw);

                        create_activity(&conn, activity).await.map_err(|e| {
                            log::error!("Failed to create Activity: {e}");
                            Status::InternalServerError
                        })?;

                        Ok(Status::Accepted)
                    } else {
                        log::error!("attributed_to does not match Actor");
                        Err(Status::Unauthorized)
                    }
                }
                _ => {
                    log::debug!("Unimplemented Update type");
                    Err(Status::NotImplemented)
                }
            },
            _ => Err(Status::UnprocessableEntity),
        }
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}
