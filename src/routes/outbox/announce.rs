use crate::routes::Outbox;
use jdt_activity_pub::{ApActivity, ApAnnounce};

use crate::{
    db::Db,
    models::{
        activities::{create_activity, NewActivity, TryFromExtendedActivity},
        actors::Actor,
        objects::get_object_by_as_id,
    },
    routes::ActivityJson,
    runner,
};
use jdt_maybe_reference::MaybeReference;
use rocket::http::Status;
use serde_json::Value;

impl Outbox for ApAnnounce {
    async fn outbox(
        &self,
        conn: Db,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        outbox(conn, self.clone(), profile, raw).await
    }
}

async fn outbox(
    conn: Db,
    announce: ApAnnounce,
    _profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, Status> {
    if let MaybeReference::Reference(as_id) = announce.clone().object {
        let object = get_object_by_as_id(Some(&conn), as_id).await.map_err(|e| {
            log::error!("FAILED TO RETRIEVE Object: {e:#?}");
            Status::NotFound
        })?;

        let mut activity = NewActivity::try_from((announce.into(), Some(object.clone().into())))
            .map_err(|e| {
                log::error!("FAILED TO BUILD Activity: {e:#?}");
                Status::InternalServerError
            })?
            .link_actor(&conn)
            .await;

        activity.raw = Some(raw);

        let activity = create_activity(Some(&conn), activity).await.map_err(|e| {
            log::error!("FAILED TO CREATE Activity: {e:#?}");
            Status::InternalServerError
        })?;

        runner::run(
            runner::announce::send_announce_task,
            conn,
            None,
            vec![activity.ap_id.clone().ok_or(Status::InternalServerError)?],
        )
        .await;

        let activity: ApActivity =
            ApActivity::try_from_extended_activity((activity, None, Some(object), None)).map_err(
                |e| {
                    log::error!("Failed to build ApActivity: {e:#?}");
                    Status::InternalServerError
                },
            )?;

        Ok(activity.into())
    } else {
        log::error!("ANNOUNCE OBJECT IS NOT A REFERENCE");
        Err(Status::new(523))
    }
}
