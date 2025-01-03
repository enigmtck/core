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
use jdt_activity_pub::{ApActivity, ApAddress, ApObject, ApUpdate};
use jdt_maybe_reference::MaybeReference;
use rocket::http::Status;
use serde_json::Value;

impl Inbox for ApUpdate {
    async fn inbox(&self, conn: Db, raw: Value) -> Result<Status, Status> {
        log::debug!("Update Message received by Inbox\n{raw:#?}");

        let activity: ApActivity = self.clone().into();

        log::debug!("ApActivity\n{activity:#?}");

        match self.clone().object {
            MaybeReference::Actual(actual) => match actual {
                ApObject::Actor(actor) => {
                    log::debug!("Updating Actor: {}", actor.clone().id.unwrap_or_default());
                    let webfinger = actor.get_webfinger().await;

                    if let Ok(mut new_remote_actor) = NewActor::try_from(actor.clone()) {
                        new_remote_actor.ek_webfinger = webfinger;

                        if actor.clone().id.unwrap_or_default() == self.actor.clone() {
                            let actor = create_or_update_actor(Some(&conn), new_remote_actor)
                                .await
                                .map_err(|e| {
                                    log::error!("Failed to create or update Actor: {e:#?}");
                                    Status::InternalServerError
                                })?;

                            let mut activity =
                                NewActivity::try_from((activity, Some(actor.into()))).map_err(
                                    |e| {
                                        log::error!("Failed to build NewActivity: {e:#?}");
                                        Status::InternalServerError
                                    },
                                )?;
                            activity.raw = Some(raw);

                            create_activity(Some(&conn), activity).await.map_err(|e| {
                                log::error!("Failed to create Activity: {e:#?}");
                                Status::InternalServerError
                            })?;

                            Ok(Status::Accepted)
                        } else {
                            log::error!("Failed to handle Activity\n{raw}");
                            Err(Status::NoContent)
                        }
                    } else {
                        log::error!("Failed to handle Activity\n{raw}");
                        Err(Status::NoContent)
                    }
                }
                ApObject::Note(note) => {
                    if let Some(id) = note.clone().id {
                        log::debug!("Updating Note: {}", id);

                        if note.clone().attributed_to == self.actor.clone() {
                            let object = create_or_update_object(&conn, note.into())
                                .await
                                .map_err(|e| {
                                    log::error!("Failed to create or update Note: {e:#?}");
                                    Status::InternalServerError
                                })?;

                            let mut activity =
                                NewActivity::try_from((activity, Some(object.into()))).map_err(
                                    |e| {
                                        log::error!("Failed to build NewActivity: {e:#?}");
                                        Status::InternalServerError
                                    },
                                )?;
                            activity.raw = Some(raw);

                            create_activity(Some(&conn), activity).await.map_err(|e| {
                                log::error!("Failed to create Activity: {e:#?}");
                                Status::InternalServerError
                            })?;

                            Ok(Status::Accepted)
                        } else {
                            log::error!("Failed to handle Activity\n{raw}");
                            Err(Status::NoContent)
                        }
                    } else {
                        log::warn!("Missing Note ID: {note:#?}");
                        log::error!("Failed to handle Activity\n{raw}");
                        Err(Status::NoContent)
                    }
                }
                ApObject::Question(question) => {
                    let id = question.clone().id;
                    log::debug!("Updating Question: {id}");

                    if question.clone().attributed_to == self.actor.clone() {
                        let object = create_or_update_object(&conn, question.into())
                            .await
                            .map_err(|e| {
                                log::error!("Failed to create or update Question: {e:#?}");
                                Status::InternalServerError
                            })?;

                        let mut activity = NewActivity::try_from((activity, Some(object.into())))
                            .map_err(|e| {
                            log::error!("Failed to build NewActivity: {e:#?}");
                            Status::InternalServerError
                        })?;
                        activity.raw = Some(raw);

                        create_activity(Some(&conn), activity).await.map_err(|e| {
                            log::error!("Failed to create Activity: {e:#?}");
                            Status::InternalServerError
                        })?;

                        Ok(Status::Accepted)
                    } else {
                        log::error!("Failed to handle Activity\n{raw}");
                        Err(Status::NoContent)
                    }
                }
                _ => {
                    log::debug!("Unimplemented Update type");
                    Err(Status::NoContent)
                }
            },
            _ => Err(Status::NoContent),
        }
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}
