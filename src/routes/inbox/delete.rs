use super::Inbox;
use jdt_activity_pub::{ApActivity, ApAddress, ApDelete, ApObject};

use crate::{
    db::Db,
    models::{
        activities::{
            create_activity, delete_activities_by_actor, revoke_activities_by_object_as_id,
            ActivityTarget, NewActivity,
        },
        actors::{get_actor_by_as_id, tombstone_actor_by_as_id},
        follows::{delete_follows_by_follower_ap_id, delete_follows_by_leader_ap_id},
        objects::{
            delete_objects_by_attributed_to, get_object_by_as_id, tombstone_object_by_as_id,
        },
        Tombstone,
    },
};
use jdt_activity_pub::MaybeMultiple;
use jdt_activity_pub::MaybeReference;
use rocket::http::Status;
use serde_json::Value;

impl Inbox for Box<ApDelete> {
    async fn inbox(&self, conn: Db, raw: Value) -> Result<Status, Status> {
        log::debug!("{:?}", self.clone());

        let tombstone = match self.object.clone() {
            MaybeReference::Actual(actual) => match actual {
                ApObject::Tombstone(tombstone) => Ok(async {
                    match get_actor_by_as_id(&conn, tombstone.id.clone()).await.ok() {
                        Some(actor) => Some(Tombstone::Actor(actor)),
                        None => get_object_by_as_id(&conn, tombstone.id.clone())
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
                        None => get_object_by_as_id(&conn, obj.id.clone())
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
                    log::error!("Failed to identify Delete Object: {}", self.object);
                    Err(Status::NoContent)
                }
            },
            MaybeReference::Reference(ap_id) => Ok(async {
                match get_actor_by_as_id(&conn, ap_id.clone()).await.ok() {
                    Some(actor) => Some(Tombstone::Actor(actor)),
                    None => get_object_by_as_id(&conn, ap_id.clone())
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

        let tombstone = tombstone.clone()?;

        let mut activity = match tombstone.clone() {
            Tombstone::Actor(actor) => NewActivity::try_from((
                ApActivity::Delete(self.clone()),
                Some(ActivityTarget::from(actor.clone())),
            ))
            .map_err(|e| {
                log::error!("Failed to build Activity: {e}");
                Status::InternalServerError
            })?,
            Tombstone::Object(object) => NewActivity::try_from((
                ApActivity::Delete(self.clone()),
                Some(ActivityTarget::from(object.clone())),
            ))
            .map_err(|e| {
                log::error!("Failed to build Activity: {e}");
                Status::InternalServerError
            })?,
        };

        activity.raw = Some(raw);

        match tombstone {
            Tombstone::Actor(actor) => {
                log::debug!("Setting Actor to Tombstone");
                if self.actor.to_string() == actor.as_id {
                    let as_id = actor.as_id;

                    log::debug!("Running database updates");
                    log::debug!("Deleting Followers: {as_id}...");
                    delete_follows_by_leader_ap_id(&conn, as_id.clone())
                        .await
                        .map_err(|e| {
                            log::error!("Failed to delete Followers: {e}");
                            Status::InternalServerError
                        })?;

                    log::debug!("Deleting Leaders: {as_id}...");
                    delete_follows_by_follower_ap_id(&conn, as_id.clone())
                        .await
                        .map_err(|e| {
                            log::error!("Failed to delete Followers by Actor: {e}");
                            Status::InternalServerError
                        })?;

                    log::debug!("Deleting Objects owned by {as_id}...");
                    delete_objects_by_attributed_to(&conn, as_id.clone())
                        .await
                        .map_err(|e| {
                            log::error!("Failed to delete Objects: {e}");
                            Status::InternalServerError
                        })?;

                    log::debug!("Deleting Activities created by {as_id}...");
                    delete_activities_by_actor(&conn, as_id.clone())
                        .await
                        .map_err(|e| {
                            log::error!("Failed to delete Activities: {e}");
                            Status::InternalServerError
                        })?;

                    tombstone_actor_by_as_id(&conn, as_id).await.map_err(|e| {
                        log::error!("Failed to delete Actor: {e}");
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
                                log::error!("Failed to delete Object: {e}");
                                Status::InternalServerError
                            })?;

                        revoke_activities_by_object_as_id(&conn, object.as_id)
                            .await
                            .map_err(|e| {
                                log::error!("Failed to revoke Activities: {e}");
                                Status::InternalServerError
                            })?;
                    }
                }
            }
        }

        let activity = create_activity(&conn, activity).await.map_err(|e| {
            log::error!("Failed to create Activity: {e}");
            Status::InternalServerError
        })?;

        log::debug!(
            "Tombstone Activity: {}",
            activity.ap_id.unwrap_or("no id".to_string())
        );

        Ok(Status::Accepted)
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}
