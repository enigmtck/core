use crate::{
    db::runner::DbRunner,
    models::{
        activities::{create_activity, NewActivity},
        actors::Actor,
        objects::{create_or_update_object, NewObject},
    },
    routes::{ActivityJson, Outbox},
    runner,
};
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::{ApActivity, ApCreate, ApObject, MaybeReference};
use reqwest::StatusCode;
use serde_json::Value;

impl Outbox for ApCreate {
    async fn outbox<C: DbRunner>(
        &self,
        conn: &C,
        pool: Pool,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        create_outbox(conn, pool, self.clone(), profile, raw).await
    }
}

async fn create_outbox<C: DbRunner>(
    conn: &C,
    pool: Pool,
    mut create: ApCreate,
    profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, StatusCode> {
    let object_to_create = match &create.object {
        MaybeReference::Actual(obj) => obj,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let new_object: NewObject = object_to_create
        .clone()
        .try_into()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let object = create_or_update_object(conn, new_object)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    create.object = match object_to_create.clone() {
        ApObject::Note(mut note) => {
            note.id = Some(object.as_id.clone());
            // TODO note.url should be a MaybeMultiple<String>
            note.url = object.as_url.clone().map(|x| x.to_string());
            MaybeReference::Actual(ApObject::Note(note))
        }
        ApObject::Question(mut question) => {
            question.id = object.as_id.clone();
            // TODO question.url should be a MaybeMultiple<String>
            question.url = object.as_url.clone().map(|x| x.to_string());
            MaybeReference::Actual(ApObject::Question(question))
        }
        ApObject::Article(mut article) => {
            article.id = Some(object.as_id.clone());
            // TODO article.url should be a MaybeMultiple<String>
            article.url = object.as_url.clone().map(|x| x.to_string());
            MaybeReference::Actual(ApObject::Article(article))
        }
        _ => MaybeReference::Actual(object_to_create.clone()),
    };

    let mut activity =
        NewActivity::try_from((ApActivity::from(create.clone()), Some(object.into())))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    activity.raw = Some(raw);

    let created_activity = create_activity(conn, activity)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    create.id = created_activity.ap_id;

    log::debug!("{create:?}");

    let final_activity = ApActivity::Create(create);

    let db_pool = pool.clone();
    let message_to_send = final_activity.clone();
    tokio::spawn(async move {
        if let Ok(conn) = db_pool.get().await {
            let inboxes = runner::user::get_follower_inboxes(&conn, profile.clone()).await;
            if let Err(e) = runner::send_to_inboxes(&conn, inboxes, profile, message_to_send).await
            {
                log::error!("Failed to send create activity: {e}");
            }
        }
    });

    Ok(ActivityJson(final_activity))
}
