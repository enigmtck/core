use super::Inbox;
use crate::{
    db::Db,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, revoke_activity_by_apid, ActivityTarget,
            NewActivity,
        },
        follows::delete_follow,
        //followers::delete_follower_by_ap_id,
    },
    runner::{self},
};
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{ApActivity, ApAddress, ApUndo};
use rocket::http::Status;
use serde_json::Value;

impl Inbox for Box<ApUndo> {
    async fn inbox(&self, conn: Db, raw: Value) -> Result<Status, Status> {
        log::debug!("{:?}", self.clone());

        match self.object.clone() {
            MaybeReference::Actual(actual) => inbox(conn, &actual, self, raw).await,
            MaybeReference::Reference(_) => {
                log::warn!("Undo object must be Actual");
                Err(Status::BadRequest)
            }
            _ => {
                log::warn!("Undo object must be Actual");
                Err(Status::BadRequest)
            }
        }
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}

async fn inbox(conn: Db, target: &ApActivity, undo: &ApUndo, raw: Value) -> Result<Status, Status> {
    let target_ap_id = target.as_id().ok_or_else(|| {
        log::error!("Activity discarded: no id");
        Status::NotImplemented
    })?;

    let (_, target_activity, _, _) = get_activity_by_ap_id(&conn, target_ap_id.clone())
        .await
        .ok_or(Status::NotFound)?;

    let activity_target = (
        ApActivity::Undo(Box::new(undo.clone())),
        Some(ActivityTarget::from(
            target_activity.ok_or(Status::NotFound)?,
        )),
    );

    let mut activity = NewActivity::try_from(activity_target).map_err(|e| {
        log::error!("Failed to build Activity: {e}");
        Status::InternalServerError
    })?;

    activity.raw = Some(raw.clone());

    create_activity(Some(&conn), activity.clone())
        .await
        .map_err(|e| {
            log::error!("Failed to create Activity: {e}");
            Status::InternalServerError
        })?;

    match target {
        ApActivity::Like(_) => {
            revoke_activity_by_apid(Some(&conn), target_ap_id.clone())
                .await
                .map_err(|e| {
                    log::error!("Failed to revoke Like: {e}");
                    Status::InternalServerError
                })?;
            Ok(Status::Accepted)
        }
        ApActivity::Follow(follow) => {
            if delete_follow(
                Some(&conn),
                follow.actor.to_string(),
                follow
                    .object
                    .reference()
                    .ok_or(Status::InternalServerError)?,
            )
            .await
            .is_ok()
                && revoke_activity_by_apid(
                    Some(&conn),
                    follow.id.clone().ok_or(Status::BadRequest)?,
                )
                .await
                .is_ok()
            {
                log::info!("Follower record deleted: {target_ap_id}");
            }

            Ok(Status::Accepted)
        }
        ApActivity::Announce(_) => {
            runner::run(
                runner::announce::remote_undo_announce_task,
                conn,
                None,
                vec![target_ap_id.clone()],
            )
            .await;
            Ok(Status::Accepted)
        }
        _ => Err(Status::NotImplemented),
    }
}
