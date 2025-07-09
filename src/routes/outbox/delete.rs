use crate::db::runner::DbRunner;
use crate::models::actors::{get_actor_by_as_id, tombstone_actor_by_as_id};
use crate::models::objects::tombstone_object_by_as_id;
use crate::routes::Outbox;
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::{ApActivity, ApDelete};
use reqwest::StatusCode;

use crate::routes::ActivityJson;
use crate::{
    models::{
        activities::{create_activity, NewActivity},
        actors::Actor,
        objects::get_object_by_as_id,
    },
    runner,
};
use serde_json::Value;

impl Outbox for Box<ApDelete> {
    async fn outbox<C: DbRunner>(
        &self,
        conn: &C,
        pool: Pool,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        delete_outbox(conn, pool, *self.clone(), profile, raw).await
    }
}

async fn delete_outbox<C: DbRunner>(
    conn: &C,
    pool: Pool,
    mut delete: ApDelete,
    profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, StatusCode> {
    let as_id = delete
        .object
        .clone()
        .reference()
        .ok_or(StatusCode::BAD_REQUEST)?;

    let mut activity = {
        if let Ok(actor) = get_actor_by_as_id(conn, as_id.clone()).await {
            tombstone_actor_by_as_id(conn, as_id).await.map_err(|e| {
                log::error!("Failure to Tombstone Actor: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            NewActivity::try_from((Box::new(delete.clone()).into(), Some(actor.clone().into())))
                .map_err(|e| {
                    log::error!("Failed to build Delete activity: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
                .link_actor(conn)
                .await
        } else if let Ok(object) = get_object_by_as_id(conn, as_id.clone()).await {
            tombstone_object_by_as_id(conn, as_id).await.map_err(|e| {
                log::error!("Failure to Tombstone Object: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            NewActivity::try_from((Box::new(delete.clone()).into(), Some(object.clone().into())))
                .map_err(|e| {
                    log::error!("Failed to build Delete activity: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
                .link_actor(conn)
                .await
        } else {
            return Err(StatusCode::NOT_FOUND);
        }
    };

    activity.raw = Some(raw);

    let created_activity = create_activity(conn, activity)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    delete.id = created_activity.ap_id;
    let final_activity = ApActivity::Delete(Box::new(delete));

    let db_pool = pool.clone();
    let message_to_send = final_activity.clone();

    let cloned_activity = final_activity.clone();
    tokio::spawn(async move {
        if let Ok(conn) = db_pool.get().await {
            let inboxes = runner::get_inboxes(&conn, cloned_activity, profile.clone()).await;
            if let Err(e) = runner::send_to_inboxes(&conn, inboxes, profile, message_to_send).await
            {
                log::error!("Failed to send delete activity: {e}");
            }
        }
    });

    Ok(ActivityJson(final_activity))
}
