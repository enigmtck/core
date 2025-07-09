use crate::{
    db::runner::DbRunner,
    models::actors::{self},
    retriever::get_actor,
    routes::{user::process_instrument, Outbox},
};
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::{
    ApActivity, ApAddress, ApArticle, ApAttachment, ApContext, ApCreate, ApImage, ApInstrument,
    ApObject, Ephemeral,
};
use reqwest::StatusCode;

use crate::{
    fairings::events::EventChannels,
    helper::{
        get_instrument_as_id_from_uuid, get_object_ap_id_from_uuid, get_object_url_from_uuid,
    },
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, Activity, NewActivity, TryFromExtendedActivity,
        },
        actors::Actor,
        cache::{cache_content, Cacheable},
        objects::{create_or_update_object, Object},
    },
    routes::ActivityJson,
    runner::{self, get_inboxes, send_to_inboxes, TaskError},
    LoadEphemeral,
};
use anyhow::Result;
use chrono::Utc;
use jdt_activity_pub::MaybeMultiple;
use serde_json::Value;
use uuid::Uuid;

impl Outbox for ApArticle {
    async fn outbox<C: DbRunner>(
        &self,
        conn: &C,
        pool: Pool,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        article_outbox(conn, pool, self.clone(), profile, raw).await
    }
}

async fn article_outbox<C: DbRunner>(
    conn: &C,
    pool: Pool,
    mut article: ApArticle,
    profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, StatusCode> {
    if article.id.is_some() {
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
        ApCreate::try_from(ApObject::Article(
            ApArticle::try_from(object.clone()).map_err(|e| {
                log::error!("Failed to build ApArticle: {e:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?,
        ))
        .map_err(|e| {
            log::error!("Failed to build ApCreate: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
    }

    fn prepare_article_metadata(article: &mut ApArticle, profile: &Actor) {
        let uuid = Uuid::new_v4().to_string();

        article.ephemeral = Some(Ephemeral {
            internal_uuid: Some(uuid.clone()),
            ..Default::default()
        });

        article.id = Some(get_object_ap_id_from_uuid(uuid.clone()));
        article.url = Some(get_object_url_from_uuid(uuid.clone()));
        article.published = Utc::now().into();
        article.attributed_to = profile.as_id.clone().into();

        article.context = Some(ApContext::default());
    }

    async fn process_instruments(mut article: ApArticle) -> Result<Vec<ApInstrument>, StatusCode> {
        let instruments = match &mut article.instrument {
            MaybeMultiple::Single(inst) => {
                let uuid = Uuid::new_v4().to_string();
                let mut cloned_inst = inst.clone();
                cloned_inst.uuid = Some(uuid.clone());
                cloned_inst.id = Some(get_instrument_as_id_from_uuid(uuid));
                vec![cloned_inst]
            }
            MaybeMultiple::Multiple(insts) => insts
                .iter()
                .map(|inst| {
                    let uuid = Uuid::new_v4().to_string();
                    let mut cloned_inst = inst.clone();
                    cloned_inst.uuid = Some(uuid.clone());
                    cloned_inst.id = Some(get_instrument_as_id_from_uuid(uuid));
                    cloned_inst
                })
                .collect::<Vec<_>>(),
            _ => vec![],
        };

        Ok(instruments)
    }

    async fn dispatch_activity<C: DbRunner>(
        _conn: &C,
        pool: Pool,
        activity: &Activity,
    ) -> Result<(), StatusCode> {
        runner::run(
            send_article,
            pool,
            None,
            vec![activity.ap_id.clone().ok_or_else(|| {
                log::error!("Activity ap_id cannot be None");
                StatusCode::INTERNAL_SERVER_ERROR
            })?],
        )
        .await;
        Ok(())
    }

    prepare_article_metadata(&mut article, &profile);

    let object = create_or_update_object(conn, (article.clone(), profile.clone()).into())
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

    for instrument in process_instruments(article.clone()).await? {
        process_instrument(conn, &profile, &instrument).await?;
    }

    let mut ap_activity =
        ApActivity::try_from_extended_activity((activity.clone(), None, Some(object), None))
            .map_err(|e| {
                log::error!("Failed to build ApActivity: {e:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    let ap_activity = ap_activity.load_ephemeral(conn, None).await;
    dispatch_activity(conn, pool, &activity).await?;

    Ok(ActivityJson(ap_activity))
}

async fn send_article(
    pool: Pool,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = pool.get().await.map_err(|_| TaskError::TaskFailed)?;

    for ap_id in ap_ids {
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

        let article = ApArticle::try_from(target_object.clone().ok_or(TaskError::TaskFailed)?)
            .map_err(|e| {
                log::error!("Failed to build ApArticle: {e:#?}");
                TaskError::TaskFailed
            })?;

        // Cache any images in the article
        if let MaybeMultiple::Multiple(attachments) = article.clone().attachment {
            for attachment in attachments {
                if let ApAttachment::Document(document) = attachment {
                    let _ = cache_content(&conn, Cacheable::try_from(ApImage::try_from(document)))
                        .await;
                }
            }
        }

        let activity = if let Ok(activity) = ApActivity::try_from_extended_activity((
            activity,
            target_activity,
            target_object,
            target_actor,
        )) {
            Some(activity.formalize())
        } else {
            log::error!("Failed to build ApActivity");
            None
        };

        let _ = get_actor(
            &conn,
            article.clone().attributed_to.to_string(),
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
        log::debug!("SENDER\n{sender:#?}");
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
