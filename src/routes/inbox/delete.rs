use super::Inbox;
use crate::activity_pub::ApDelete;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApObject},
    db::Db,
    models::{
        activities::{
            create_activity, revoke_activities_by_object_as_id, ActivityTarget, NewActivity,
        },
        actors::{get_actor_by_as_id, tombstone_actor_by_as_id},
        objects::{get_object_by_as_id, tombstone_object_by_as_id},
        Tombstone,
    },
    MaybeMultiple, MaybeReference,
};
use rocket::http::Status;
use serde_json::Value;

impl Inbox for Box<ApDelete> {
    async fn inbox(&self, conn: Db, raw: Value) -> Result<Status, Status> {
        log::debug!("Delete Message received by Inbox\n{raw:#?}");

        let tombstone = match self.object.clone() {
            MaybeReference::Actual(actual) => match actual {
                ApObject::Tombstone(tombstone) => Ok(async {
                    match get_actor_by_as_id(&conn, tombstone.id.clone()).await.ok() {
                        Some(actor) => Some(Tombstone::Actor(actor)),
                        None => get_object_by_as_id(Some(&conn), tombstone.id.clone())
                            .await
                            .ok()
                            .map(Tombstone::Object),
                    }
                }
                .await
                .ok_or_else(|| {
                    log::debug!("Failed to identify Tombstone: {}", tombstone.id);
                    Status::NotFound
                })?),
                ApObject::Identifier(obj) => Ok(async {
                    match get_actor_by_as_id(&conn, obj.id.clone()).await.ok() {
                        Some(actor) => Some(Tombstone::Actor(actor)),
                        None => get_object_by_as_id(Some(&conn), obj.id.clone())
                            .await
                            .ok()
                            .map(Tombstone::Object),
                    }
                }
                .await
                .ok_or_else(|| {
                    log::debug!("Failed to determine Identifier: {}", obj.id);
                    Status::NotFound
                })?),
                _ => {
                    log::error!("Failed to identify Delete Object: {:#?}", self.object);
                    Err(Status::NoContent)
                }
            },
            MaybeReference::Reference(ap_id) => Ok(async {
                match get_actor_by_as_id(&conn, ap_id.clone()).await.ok() {
                    Some(actor) => Some(Tombstone::Actor(actor)),
                    None => get_object_by_as_id(Some(&conn), ap_id.clone())
                        .await
                        .ok()
                        .map(Tombstone::Object),
                }
            }
            .await
            .ok_or_else(|| {
                log::debug!("Failed to identify Tombstone");
                Status::NotFound
            })?),
            _ => {
                log::debug!("Not implemented: MaybeReference not Actual or Reference");
                Err(Status::NotImplemented)
            }
        };

        log::debug!("Tombstone\n{tombstone:#?}");
        let tombstone = tombstone.clone()?;

        let mut activity = match tombstone.clone() {
            Tombstone::Actor(actor) => NewActivity::try_from((
                ApActivity::Delete(self.clone()),
                Some(ActivityTarget::from(actor.clone())),
            ))
            .map_err(|e| {
                log::error!("Failed to build Activity: {e:#?}");
                Status::InternalServerError
            })?,
            Tombstone::Object(object) => NewActivity::try_from((
                ApActivity::Delete(self.clone()),
                Some(ActivityTarget::from(object.clone())),
            ))
            .map_err(|e| {
                log::error!("Failed to build Activity: {e:#?}");
                Status::InternalServerError
            })?,
        };

        activity.raw = Some(raw);

        let activity = create_activity(Some(&conn), activity).await.map_err(|e| {
            log::error!("Failed to create Activity: {e:#?}");
            Status::InternalServerError
        })?;

        log::debug!("Tombstone Activity\n{activity:#?}");

        match tombstone {
            Tombstone::Actor(actor) => {
                log::debug!("Setting Actor to Tombstone");
                if self.actor.to_string() == actor.as_id {
                    log::debug!("Running database updates");
                    tombstone_actor_by_as_id(&conn, actor.as_id)
                        .await
                        .map_err(|e| {
                            log::error!("Failed to delete Actor: {e:#?}");
                            Status::InternalServerError
                        })?;
                }
            }
            Tombstone::Object(object) => {
                log::debug!("Setting Object to Tombstone");
                if let Some(attributed_to) = object.as_attributed_to {
                    let attributed_to: MaybeMultiple<ApAddress> = attributed_to.into();
                    let attributed_to = attributed_to.single().map_err(|e| {
                        log::error!("{e}");
                        Status::InternalServerError
                    })?;

                    if self.actor.to_string() == attributed_to.clone().to_string() {
                        log::debug!("Running database updates");
                        tombstone_object_by_as_id(&conn, object.as_id.clone())
                            .await
                            .map_err(|e| {
                                log::error!("Failed to delete Object: {e:#?}");
                                Status::InternalServerError
                            })?;

                        revoke_activities_by_object_as_id(&conn, object.as_id)
                            .await
                            .map_err(|e| {
                                log::error!("Failed to revoke Activities: {e:#?}");
                                Status::InternalServerError
                            })?;
                    }
                }
            }
        }

        Ok(Status::Accepted)
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}
