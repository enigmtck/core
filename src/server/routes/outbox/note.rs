use super::ActivityJson;
use crate::server::routes::Outbox;
use crate::{
    db::runner::DbRunner,
    events::EventChannels,
    helper::{
        get_conversation_ap_id_from_uuid, get_instrument_as_id_from_uuid,
        get_object_ap_id_from_uuid, get_object_url_from_uuid,
    },
    models::actors::{self},
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, NewActivity, TryFromExtendedActivity,
        },
        actors::Actor,
        cache::{cache_content, Cacheable},
        objects::{create_object, Object},
    },
    retriever::get_actor,
    runner::{self, get_inboxes, send_to_inboxes, TaskError},
    server::routes::user::process_instrument,
    LoadEphemeral,
};
use anyhow::Result;
use chrono::Utc;
use deadpool_diesel::postgres::Pool;
use jdt_activity_pub::MaybeMultiple;
use jdt_activity_pub::{
    ApActivity, ApAddress, ApAttachment, ApContext, ApCreate, ApImage, ApInstrument, ApNote,
    ApNoteType, ApObject, ApUrl, Ephemeral,
};
use reqwest::StatusCode;
use serde_json::Value;
use uuid::Uuid;

impl Outbox for ApNote {
    async fn outbox<C: DbRunner>(
        &self,
        conn: &C,
        pool: Pool,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        note_outbox(conn, pool, self.clone(), profile, raw).await
    }
}

async fn note_outbox<C: DbRunner>(
    conn: &C,
    pool: Pool,
    mut note: ApNote,
    profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, StatusCode> {
    if note.id.is_some() {
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
        ApCreate::try_from(ApObject::Note(ApNote::try_from(object.clone()).map_err(
            |e| {
                log::error!("Failed to build ApNote: {e:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            },
        )?))
        .map_err(|e| {
            log::error!("Failed to build ApCreate: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
    }

    fn prepare_note_metadata(note: &mut ApNote, profile: &Actor) {
        let uuid = Uuid::new_v4().to_string();

        note.ephemeral = Some(Ephemeral {
            internal_uuid: Some(uuid.clone()),
            ..Default::default()
        });

        note.id = Some(get_object_ap_id_from_uuid(uuid.clone()));
        note.url = MaybeMultiple::Single(ApUrl::from(get_object_url_from_uuid(uuid.clone())));
        note.published = Utc::now().into();
        note.attributed_to = profile.as_id.clone().into();

        if note.conversation.is_none() {
            note.conversation = Some(get_conversation_ap_id_from_uuid(Uuid::new_v4().to_string()));
        }

        note.context = Some(ApContext::default());
    }

    async fn process_instruments(mut note: ApNote) -> Result<Vec<ApInstrument>, StatusCode> {
        let instruments = match &mut note.instrument {
            MaybeMultiple::Single(inst) => {
                let uuid = Uuid::new_v4().to_string();
                let mut cloned_inst = inst.clone();
                cloned_inst.uuid = Some(uuid.clone());
                cloned_inst.id = Some(get_instrument_as_id_from_uuid(uuid));
                cloned_inst.conversation = note.conversation.clone();
                vec![cloned_inst]
            }
            MaybeMultiple::Multiple(insts) => insts
                .iter()
                .map(|inst| {
                    let uuid = Uuid::new_v4().to_string();
                    let mut cloned_inst = inst.clone();
                    cloned_inst.uuid = Some(uuid.clone());
                    cloned_inst.id = Some(get_instrument_as_id_from_uuid(uuid));
                    cloned_inst.conversation = note.conversation.clone();
                    cloned_inst
                })
                .collect::<Vec<_>>(),
            _ => vec![],
        };

        Ok(instruments)
    }

    prepare_note_metadata(&mut note, &profile);

    let object = create_object(conn, (note.clone(), profile.clone()).into())
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

    for instrument in process_instruments(note.clone()).await? {
        process_instrument(conn, &profile, &instrument).await?;
    }

    let mut ap_activity =
        ApActivity::try_from_extended_activity((activity.clone(), None, Some(object), None))
            .map_err(|e| {
                log::error!("Failed to build ApActivity: {e:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    let ap_activity = ap_activity.load_ephemeral(conn, None).await;

    let pool = pool.clone();
    let ap_id = activity.ap_id.clone().ok_or_else(|| {
        log::error!("Activity ap_id cannot be None");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    runner::run(send_note, pool, None, vec![ap_id]).await;

    Ok(ActivityJson(ap_activity))
}

async fn send_note(
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

        let note = ApNote::try_from(target_object.clone()).map_err(|e| {
            log::error!("Failed to build ApNote: {e:#?}");
            TaskError::TaskFailed
        })?;

        log::debug!("Note Name: {:?}", note.name);

        // For the Svelte client, all images are passed through the server cache. To cache an image
        // that's already on the server seems weird, but I think it's a better choice than trying
        // to handle the URLs for local images differently.
        //let ap_note: ApNote = note.clone().into();
        if let MaybeMultiple::Multiple(attachments) = note.clone().attachment {
            for attachment in attachments {
                if let ApAttachment::Document(document) = attachment {
                    let _ = cache_content(&conn, Cacheable::try_from(ApImage::try_from(document)))
                        .await;
                }
            }
        }

        cfg_if::cfg_if! {
            if #[cfg(feature = "pg")] {
                let activity = match note.kind {
                    ApNoteType::Note | ApNoteType::EncryptedNote => {
                        if let Ok(activity) =
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
                            }
                    }
                    _ => None,
                };
            } else if #[cfg(feature = "sqlite")] {
                let activity = {
                    if note.kind.to_lowercase().as_str() == "note" {
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
                    } else {
                        // NoteType::EncryptedNote => {
                        //     handle_encrypted_note(&mut note, sender.clone())
                        //         .map(ApActivity::Create(ApCreate::from))
                        // }
                        None
                    }
                };
            }
        }

        let _ = get_actor(
            &conn,
            note.clone().attributed_to.to_string(),
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
        //log::debug!("SENDER\n{sender:#?}");
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
