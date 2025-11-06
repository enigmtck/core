use crate::{
    db::runner::DbRunner,
    helper::{
        get_conversation_ap_id_from_uuid, get_object_ap_id_from_uuid, get_object_url_from_uuid,
    },
    models::{activities::NewActivity, actors::Actor, objects::Object},
    server::routes::{ActivityJson, Outbox},
};
use chrono::Utc;
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::{
    ApActivity, ApContext, ApCreate, ApObject, ApQuestion, ApUrl, Ephemeral, MaybeMultiple,
};
use reqwest::StatusCode;
use serde_json::Value;
use uuid::Uuid;

impl Outbox for ApQuestion {
    async fn outbox<C: DbRunner>(
        &self,
        conn: &C,
        pool: Pool,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        question_outbox(conn, pool, self.clone(), profile, raw).await
    }
}

async fn question_outbox<C: DbRunner>(
    conn: &C,
    pool: Pool,
    mut question: ApQuestion,
    profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, StatusCode> {
    if question.id.is_some() {
        return Err(StatusCode::NOT_ACCEPTABLE);
    }

    async fn build_activity<C: DbRunner>(
        create: ApCreate,
        conn: &C,
        object: &Object,
        raw: Value,
    ) -> Result<NewActivity, StatusCode> {
        let mut activity = NewActivity::try_from((create.into(), Some(object.into())))
            .map_err(|e| {
                log::error!("Failed to build Activity: {e:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .link_actor(conn)
            .await;

        activity.raw = Some(raw);

        Ok(activity)
    }

    fn build_ap_create(object: &Object) -> Result<ApCreate, StatusCode> {
        ApCreate::try_from(ApObject::Question(
            ApQuestion::try_from(object.clone()).map_err(|e| {
                log::error!("Failed to build ApNote: {e:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?,
        ))
        .map_err(|e| {
            log::error!("Failed to build ApCreate: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
    }

    fn prepare_note_metadata(question: &mut ApQuestion, profile: &Actor) {
        let uuid = Uuid::new_v4().to_string();

        question.ephemeral = Some(Ephemeral {
            internal_uuid: Some(uuid.clone()),
            ..Default::default()
        });

        question.id = Some(get_object_ap_id_from_uuid(uuid.clone()));
        question.url = MaybeMultiple::Single(ApUrl::from(get_object_url_from_uuid(uuid.clone())));
        question.published = Some(Utc::now().into());
        question.attributed_to = profile.as_id.clone().into();

        if question.conversation.is_none() {
            question.conversation =
                Some(get_conversation_ap_id_from_uuid(Uuid::new_v4().to_string()));
        }

        question.context = Some(ApContext::default());
    }

    return Err(StatusCode::NOT_ACCEPTABLE);
}
