use crate::server::AppState;
use crate::{
    db::runner::DbRunner,
    events::EventChannels,
    helper::{
        get_conversation_ap_id_from_uuid, get_object_ap_id_from_uuid, get_object_url_from_uuid,
    },
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, NewActivity, TryFromExtendedActivity,
        },
        actors::{self, Actor},
        objects::{create_object, Object},
    },
    retriever::get_actor,
    runner::{self, get_inboxes, send_to_inboxes, TaskError},
    server::routes::{ActivityJson, Outbox},
    LoadEphemeral,
};
use anyhow::Result;
use chrono::Utc;
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::{
    ApActivity, ApAddress, ApContext, ApCreate, ApObject, ApQuestion, ApUrl, Ephemeral,
    MaybeMultiple,
};
use reqwest::StatusCode;
use serde_json::Value;
use uuid::Uuid;

impl Outbox for ApQuestion {
    async fn outbox<C: DbRunner>(
        &self,
        conn: &C,
        state: AppState,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        question_outbox(conn, state, self.clone(), profile, raw).await
    }
}

async fn question_outbox<C: DbRunner>(
    conn: &C,
    state: AppState,
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

    fn prepare_question_metadata(question: &mut ApQuestion, profile: &Actor) {
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

    prepare_question_metadata(&mut question, &profile);

    let object = create_object(conn, (question.clone(), profile.clone()).into())
        .await
        .map_err(|e| {
            log::error!("Failed to create or update Object: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let create = build_ap_create(&object)?;

    let activity = create_activity(conn, build_activity(create, conn, &object, raw).await?)
        .await
        .map_err(|e| {
            log::error!("Failed to create Activity: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut ap_activity =
        ApActivity::try_from_extended_activity((activity.clone(), None, Some(object), None))
            .map_err(|e| {
                log::error!("Failed to build ApActivity: {e:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    let ap_activity = ap_activity.load_ephemeral(conn, None).await;

    let pool = state.db_pool.clone();
    let ap_id = activity.ap_id.clone().ok_or_else(|| {
        log::error!("Activity ap_id cannot be None");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    runner::run(send_question, pool, None, vec![ap_id]).await;

    Ok(ActivityJson(ap_activity))
}

async fn send_question(
    pool: Pool,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    for ap_id in ap_ids {
        let conn = pool.get().await.map_err(|_| TaskError::TaskFailed)?;
        let (activity, target_activity, target_object, target_actor) =
            get_activity_by_ap_id(&conn, ap_id.clone())
                .await
                .map_err(|e| {
                    log::error!("Failed to retrieve Activity: {e}");
                    TaskError::TaskFailed
                })?
                .ok_or_else(|| {
                    log::error!("Activity not found: {ap_id}");
                    TaskError::TaskFailed
                })?;

        let profile_id = activity.actor_id.ok_or_else(|| {
            log::error!("Activity actor_id cannot be None");
            TaskError::TaskFailed
        })?;

        let sender = actors::get_actor(&conn, profile_id).await.map_err(|e| {
            log::error!("Failed to retrieve sending Actor: {e}");
            TaskError::TaskFailed
        })?;

        let target_object = target_object.clone().ok_or(TaskError::TaskFailed)?;

        log::debug!("Object Name: {:?}", target_object.as_name.clone());

        let question = ApQuestion::try_from(target_object.clone()).map_err(|e| {
            log::error!("Failed to build ApQuestion: {e:#?}");
            TaskError::TaskFailed
        })?;

        log::debug!("Question Content: {:?}", question.content);

        cfg_if::cfg_if! {
            if #[cfg(feature = "pg")] {
                let activity = if let Ok(activity) =
                    ApActivity::try_from_extended_activity((
                        activity,
                        target_activity,
                        Some(target_object),
                        target_actor
                    )) {
                        Some(activity.formalize())
                    } else {
                        log::error!("Failed to build ApActivity");
                        None
                    };
            } else if #[cfg(feature = "sqlite")] {
                let activity = {
                    if let Ok(activity) = ApActivity::try_from((
                        (
                            activity,
                            target_activity,
                            target_object
                        ),
                        None,
                    )) {
                        Some(activity.formalize())
                    } else {
                        None
                    }
                };
            }
        }

        let _ = get_actor(
            &conn,
            question.clone().attributed_to.to_string(),
            Some(sender.clone()),
            true,
        )
        .await;

        let activity = activity.ok_or_else(|| {
            log::error!("Activity cannot be None");
            TaskError::TaskFailed
        })?;

        let inboxes: Vec<ApAddress> = get_inboxes(&conn, activity.clone(), sender.clone()).await;

        log::debug!("SENDING ACTIVITY\n{activity:#?}");
        log::debug!("INBOXES\n{inboxes:#?}");

        send_to_inboxes(&conn, inboxes, sender, activity)
            .await
            .map_err(|e| {
                log::error!("Failed to send ApActivity: {e:#?}");
                TaskError::TaskFailed
            })?;
    }

    Ok(())
}
