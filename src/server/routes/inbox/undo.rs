use super::Inbox;
use crate::{
    db::runner::DbRunner,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, revoke_activity_by_apid, ActivityTarget,
            ActivityType, NewActivity,
        },
        follows::delete_follow,
    },
    runner::{self},
};
use deadpool_diesel::postgres::Pool;
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

        let target_ap_id = self.object.reference().ok_or_else(|| {
            log::warn!("Unable to determine Undo target Object");
            StatusCode::BAD_REQUEST
        })?;

        let (_, target_activity, _, _) = get_activity_by_ap_id(conn, target_ap_id.clone())
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?
            .ok_or_else(|| {
                log::error!("Undo Activity not found");
                StatusCode::NOT_FOUND
            })?;

        let target_activity = target_activity.ok_or_else(|| {
            log::error!("Target Activity not found");
            StatusCode::NOT_FOUND
        })?;

        let activity_target = (
            ApActivity::Undo(self.clone()),
            Some(ActivityTarget::from(target_activity.clone())),
        );

        let activity = NewActivity::try_from(activity_target)
            .map_err(|e| {
                log::error!("Failed to build Activity: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .set_raw(raw);

        create_activity(conn, activity.clone()).await.map_err(|e| {
            log::error!("Failed to create Activity: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        match target_activity.kind {
            ActivityType::Like => {
                revoke_activity_by_apid(conn, target_ap_id.clone())
                    .await
                    .map_err(|e| {
                        log::error!("Failed to revoke Like: {e}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;
                Ok(StatusCode::ACCEPTED)
            }
            ActivityType::Follow => {
                let followed_actor_ap_id = target_activity.target_ap_id.ok_or_else(|| {
                    log::error!("Failed to identify followed Actor");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

                let follow_activity_ap_id = target_activity.ap_id.clone().ok_or_else(|| {
                    log::error!("Failed to identify Follow Activity");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

                if delete_follow(
                    conn,
                    target_activity.actor.to_string(),
                    followed_actor_ap_id,
                )
                .await
                .is_ok()
                    && revoke_activity_by_apid(conn, follow_activity_ap_id)
                        .await
                        .is_ok()
                {
                    log::info!("Follower record deleted: {target_ap_id}");
                }

                Ok(StatusCode::ACCEPTED)
            }
            ActivityType::Announce => {
                let pool = pool.clone();
                let ap_id = target_ap_id.clone();

                tokio::spawn(async move {
                    if let Err(e) =
                        runner::announce::remote_undo_announce_task(pool, None, vec![ap_id]).await
                    {
                        log::error!("Failed to run remote_undo_announce_task: {e:?}");
                    }
                });
                Ok(StatusCode::ACCEPTED)
            }
            _ => Err(StatusCode::NOT_IMPLEMENTED),
        }
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}
