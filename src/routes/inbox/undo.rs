use super::Inbox;
use crate::{
    db::Db,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, revoke_activity_by_apid, ActivityTarget,
            NewActivity,
        },
        followers::delete_follower_by_ap_id,
    },
    runner::{self},
};
use jdt_activity_pub::{ApActivity, ApAddress, ApUndo};
use jdt_maybe_reference::MaybeReference;
use rocket::http::Status;
use serde_json::Value;

impl Inbox for Box<ApUndo> {
    async fn inbox(&self, conn: Db, raw: Value) -> Result<Status, Status> {
        match self.object.clone() {
            MaybeReference::Actual(actual) => inbox(conn, &actual, self, raw).await,
            MaybeReference::Reference(_) => {
                log::warn!(
                    "INSUFFICIENT CONTEXT FOR UNDO TARGET (REFERENCE FOUND WHEN ACTUAL IS REQUIRED)"
                );
                log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                Err(Status::BadRequest)
            }
            _ => {
                log::warn!(
                    "INSUFFICIENT CONTEXT FOR UNDO TARGET (NONE FOUND WHEN ACTUAL IS REQUIRED)"
                );
                log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
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
        log::error!("ApActivity.as_id() FAILED\n{target:#?}");
        Status::NotImplemented
    })?;

    let (_, target_activity, _, _) = get_activity_by_ap_id(&conn, target_ap_id.clone())
        .await
        .ok_or({
            log::error!("ACTIVITY NOT FOUND: {target_ap_id}");
            Status::NotFound
        })?;

    let activity_target = (
        ApActivity::Undo(Box::new(undo.clone())),
        Some(ActivityTarget::from(
            target_activity.ok_or(Status::NotFound)?,
        )),
    );

    let mut activity = NewActivity::try_from(activity_target).map_err(|e| {
        log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
        Status::InternalServerError
    })?;

    activity.raw = Some(raw.clone());

    create_activity(Some(&conn), activity.clone())
        .await
        .map_err(|e| {
            log::error!("FAILED TO CREATE ACTIVITY: {e:#?}");
            Status::InternalServerError
        })?;

    match target {
        ApActivity::Like(_) => {
            revoke_activity_by_apid(Some(&conn), target_ap_id.clone())
                .await
                .map_err(|e| {
                    log::error!("FAILED TO REVOKE LIKE: {e:#?}");
                    Status::InternalServerError
                })?;
            Ok(Status::Accepted)
        }
        ApActivity::Follow(_) => {
            if delete_follower_by_ap_id(Some(&conn), target_ap_id.clone()).await {
                log::info!("FOLLOWER RECORD DELETED: {target_ap_id}");
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
