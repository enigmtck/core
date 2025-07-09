use crate::{db::runner::DbRunner, routes::Outbox};
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::{ApActivity, ApAnnounce};
use reqwest::StatusCode;

use crate::{
    models::{
        activities::{create_activity, NewActivity, TryFromExtendedActivity},
        actors::Actor,
        objects::get_object_by_as_id,
    },
    routes::ActivityJson,
    runner,
};
use jdt_activity_pub::MaybeReference;
use serde_json::Value;

impl Outbox for ApAnnounce {
    async fn outbox<C: DbRunner>(
        &self,
        conn: &C,
        pool: Pool,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        announce_outbox(conn, pool, self.clone(), profile, raw).await
    }
}

async fn announce_outbox<C: DbRunner>(
    conn: &C,
    pool: Pool,
    announce: ApAnnounce,
    _profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, StatusCode> {
    if let MaybeReference::Reference(as_id) = announce.clone().object {
        let object = get_object_by_as_id(conn, as_id).await.map_err(|e| {
            log::error!("FAILED TO RETRIEVE Object: {e:#?}");
            StatusCode::NOT_FOUND
        })?;

        let mut activity = NewActivity::try_from((announce.into(), Some(object.clone().into())))
            .map_err(|e| {
                log::error!("FAILED TO BUILD Activity: {e:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .link_actor(conn)
            .await;

        activity.raw = Some(raw);

        let activity = create_activity(conn, activity).await.map_err(|e| {
            log::error!("FAILED TO CREATE Activity: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        runner::run(
            runner::announce::send_announce_task,
            pool,
            None,
            vec![activity
                .ap_id
                .clone()
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?],
        )
        .await;

        let activity: ApActivity =
            ApActivity::try_from_extended_activity((activity, None, Some(object), None)).map_err(
                |e| {
                    log::error!("Failed to build ApActivity: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                },
            )?;

        Ok(ActivityJson(activity))
    } else {
        log::error!("ANNOUNCE OBJECT IS NOT A REFERENCE");
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}
