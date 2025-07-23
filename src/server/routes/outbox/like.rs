use super::ActivityJson;
use crate::db::runner::DbRunner;
use crate::server::routes::Outbox;
use crate::{
    models::{
        activities::{create_activity, NewActivity},
        actors::Actor,
        objects::get_object_by_as_id,
    },
    runner,
};
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::{ApActivity, ApLike};
use reqwest::StatusCode;
use serde_json::Value;

impl Outbox for Box<ApLike> {
    async fn outbox<C: DbRunner>(
        &self,
        conn: &C,
        pool: Pool,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        like_outbox(conn, pool, *self.clone(), profile, raw).await
    }
}

async fn like_outbox<C: DbRunner>(
    conn: &C,
    pool: Pool,
    mut like: ApLike,
    _profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, StatusCode> {
    let as_id = like.object.reference().ok_or(StatusCode::BAD_REQUEST)?;

    let object = get_object_by_as_id(conn, as_id).await.map_err(|e| {
        log::error!("Failed to retrieve Object: {e:#?}");
        StatusCode::NOT_FOUND
    })?;

    // Ensure the 'to' field is set to the author of the liked object.
    like.to = object.as_attributed_to.clone().into();

    let mut activity = NewActivity::try_from((
        ApActivity::from(Box::new(like.clone())),
        Some(object.into()),
    ))
    .map_err(|e| {
        log::error!("Failed to build Activity: {e:#?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .link_actor(conn)
    .await;

    activity.raw = Some(raw);

    let created_activity = create_activity(conn, activity.clone()).await.map_err(|e| {
        log::error!("Failed to create Activity: {e:#?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    like.id = created_activity.ap_id.clone();

    let ap_id = created_activity.ap_id.clone().ok_or_else(|| {
        log::error!("ActivityPub ID cannot be None for federation");
        StatusCode::BAD_REQUEST
    })?;

    let final_activity = ApActivity::Like(Box::new(like));

    runner::run(runner::send_activity_task, pool, None, vec![ap_id]).await;

    // let db_pool = pool.clone();
    // let message_to_send = final_activity.clone();
    // tokio::spawn(async move {
    //     if let Ok(conn) = db_pool.get().await {
    //         let inboxes =
    //             runner::get_inboxes(&conn, message_to_send.clone(), profile.clone()).await;
    //         if let Err(e) = runner::send_to_inboxes(&conn, inboxes, profile, message_to_send).await
    //         {
    //             log::error!("Failed to send Like Activity: {e}");
    //         }
    //     }
    // });

    Ok(ActivityJson(final_activity))
}
