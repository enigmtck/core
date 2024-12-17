use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActor, ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox},
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::{create_activity, ApActivityTarget, NewActivity},
        actors::{create_or_update_actor, Actor, NewActor},
        objects::create_or_update_object,
    },
    routes::ActivityJson,
    MaybeMultiple, MaybeReference,
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{activity::ApActivity, signature::ApSignature};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApUpdateType {
    #[default]
    #[serde(alias = "update")]
    Update,
}

impl fmt::Display for ApUpdateType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApUpdate {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApUpdateType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
    pub signature: Option<ApSignature>,
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub to: MaybeMultiple<ApAddress>,
}

impl Inbox for ApUpdate {
    async fn inbox(
        &self,
        conn: Db,
        _channels: EventChannels,
        raw: Value,
    ) -> Result<Status, Status> {
        let activity: ApActivity = self.clone().into();

        match self.clone().object {
            MaybeReference::Actual(actual) => match actual {
                ApObject::Actor(actor) => {
                    log::debug!("UPDATING ACTOR: {}", actor.clone().id.unwrap_or_default());
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
                            log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                            Err(Status::NoContent)
                        }
                    } else {
                        log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                        Err(Status::NoContent)
                    }
                }
                ApObject::Note(note) => {
                    if let Some(id) = note.clone().id {
                        log::debug!("UPDATING NOTE: {}", id);

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
                            log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                            Err(Status::NoContent)
                        }
                    } else {
                        log::warn!("MISSING NOTE ID: {note:#?}");
                        log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                        Err(Status::NoContent)
                    }
                }
                ApObject::Question(question) => {
                    let id = question.clone().id;
                    log::debug!("UPDATING QUESTION: {id}");

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
                        log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                        Err(Status::NoContent)
                    }
                }
                _ => {
                    log::debug!("UNIMPLEMENTED UPDATE TYPE");
                    log::error!("{raw:#?}");
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

impl Outbox for ApUpdate {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Actor,
        _raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl TryFrom<ApNote> for ApUpdate {
    type Error = &'static str;

    fn try_from(note: ApNote) -> Result<Self, Self::Error> {
        if let Some(id) = note.id.clone() {
            Ok(ApUpdate {
                context: Some(ApContext::default()),
                actor: note.attributed_to.clone(),
                kind: ApUpdateType::default(),
                id: Some(format!("{id}#update")),
                object: MaybeReference::Actual(ApObject::Note(note.clone())),
                signature: None,
                to: note.to,
            })
        } else {
            Err("ApNote must have an ID")
        }
    }
}

impl TryFrom<ApActor> for ApUpdate {
    type Error = &'static str;

    fn try_from(actor: ApActor) -> Result<Self, Self::Error> {
        if let Some(id) = actor.id.clone() {
            Ok(ApUpdate {
                context: Some(ApContext::default()),
                actor: id.clone(),
                kind: ApUpdateType::default(),
                id: Some(format!("{id}#update")),
                object: MaybeReference::Actual(ApObject::Actor(actor)),
                signature: None,
                to: vec![ApAddress::get_public()].into(),
            })
        } else {
            Err("ApActor must have an ID")
        }
    }
}
