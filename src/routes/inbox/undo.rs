use super::Inbox;
use crate::{
    db::runner::DbRunner,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, revoke_activity_by_apid, ActivityTarget,
            NewActivity,
        },
        follows::delete_follow,
    },
    runner::{self},
};
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{ApActivity, ApAddress, ApUndo};
use reqwest::StatusCode;
use serde_json::Value;

impl Inbox for Box<ApUndo> {
    async fn inbox<C: DbRunner>(
        &self,
        conn: &C,
        pool: Pool,
        raw: Value,
    ) -> Result<StatusCode, StatusCode> {
        log::debug!("{:?}", self.clone());

        match self.object.clone() {
            MaybeReference::Actual(actual) => inbox(conn, pool, &actual, self, raw).await,
            MaybeReference::Reference(_) => {
                log::warn!("Undo object must be Actual");
                Err(StatusCode::BAD_REQUEST)
            }
            _ => {
                log::warn!("Undo object must be Actual");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}

async fn inbox<C: DbRunner>(
    conn: &C,
    pool: Pool,
    target: &ApActivity,
    undo: &ApUndo,
    raw: Value,
) -> Result<StatusCode, StatusCode> {
    let target_ap_id = target.as_id().ok_or_else(|| {
        log::error!("Activity discarded: no id");
        StatusCode::NOT_IMPLEMENTED
    })?;

    let (_, target_activity, _, _) = get_activity_by_ap_id(conn, target_ap_id.clone())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?
        .ok_or_else(|| {
            log::error!("Activity not found");
            StatusCode::NOT_FOUND
        })?;

    let activity_target = (
        ApActivity::Undo(Box::new(undo.clone())),
        Some(ActivityTarget::from(
            target_activity.ok_or(StatusCode::NOT_FOUND)?,
        )),
    );

    let mut activity = NewActivity::try_from(activity_target).map_err(|e| {
        log::error!("Failed to build Activity: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    activity.raw = Some(raw.clone());

    create_activity(conn, activity.clone()).await.map_err(|e| {
        log::error!("Failed to create Activity: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match target {
        ApActivity::Like(_) => {
            revoke_activity_by_apid(conn, target_ap_id.clone())
                .await
                .map_err(|e| {
                    log::error!("Failed to revoke Like: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            Ok(StatusCode::ACCEPTED)
        }
        ApActivity::Follow(follow) => {
            if delete_follow(
                conn,
                follow.actor.to_string(),
                follow
                    .object
                    .reference()
                    .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            )
            .await
            .is_ok()
                && revoke_activity_by_apid(conn, follow.id.clone().ok_or(StatusCode::BAD_REQUEST)?)
                    .await
                    .is_ok()
            {
                log::info!("Follower record deleted: {target_ap_id}");
            }

            Ok(StatusCode::ACCEPTED)
        }
        ApActivity::Announce(_) => {
            runner::run(
                runner::announce::remote_undo_announce_task,
                pool,
                None,
                vec![target_ap_id.clone()],
            )
            .await;
            Ok(StatusCode::ACCEPTED)
        }
        _ => Err(StatusCode::NOT_IMPLEMENTED),
    }
}
