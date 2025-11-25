use super::ActivityJson;
use crate::db::runner::DbRunner;
use crate::models::actors::get_actor_by_as_id;
use crate::server::routes::Outbox;
use crate::server::AppState;
use crate::{
    models::{
        activities::{create_activity, NewActivity},
        actors::Actor,
        objects::get_object_by_as_id,
    },
    runner,
};
use jdt_activity_pub::{ApActivity, ApDelete};
use reqwest::StatusCode;
use serde_json::Value;

impl Outbox for Box<ApDelete> {
    async fn outbox<C: DbRunner>(
        &self,
        conn: &C,
        state: AppState,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        delete_outbox(conn, state, *self.clone(), profile, raw).await
    }
}

async fn delete_outbox<C: DbRunner>(
    conn: &C,
    state: AppState,
    mut delete: ApDelete,
    _profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, StatusCode> {
    let as_id = delete
        .object
        .clone()
        .reference()
        .ok_or(StatusCode::BAD_REQUEST)?;

    let mut activity = {
        if let Ok(actor) = get_actor_by_as_id(conn, as_id.clone()).await {
            NewActivity::try_from((Box::new(delete.clone()).into(), Some(actor.clone().into())))
                .map_err(|e| {
                    log::error!("Failed to build Delete activity: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
                .link_actor(conn)
                .await
        } else if let Ok(object) = get_object_by_as_id(conn, as_id.clone()).await {
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

    delete.id = created_activity.ap_id.clone();
    let ap_id = created_activity.ap_id.ok_or_else(|| {
        log::error!("Activity ap_id cannot be None");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    runner::run(runner::send_activity_task, state.db_pool, None, vec![ap_id]).await;

    let final_activity = ApActivity::Delete(Box::new(delete));

    Ok(ActivityJson(final_activity))
}
