use anyhow::Result;
use std::collections::HashSet;

use crate::{
    activity_pub::{
        ApActivity, ApActor, ApInstrument, ApInstrumentType, ApInstruments, ApInvite, ApJoin,
        ApNote, ApSession, JoinData,
    },
    db::Db,
    fairings::events::EventChannels,
    helper::{get_local_identifier, is_local, LocalIdentifierType},
    models::{
        actors::{get_actor, get_actor_by_as_id, get_actor_by_username, Actor},
        encrypted_sessions::{
            create_encrypted_session, get_encrypted_session_by_profile_id_and_ap_to,
            get_encrypted_session_by_uuid, NewEncryptedSession,
        },
        objects::Object,
        olm_one_time_keys::get_olm_one_time_key_by_profile_id,
        olm_sessions::{create_olm_session, update_olm_session},
        processing_queue::create_processing_item,
        remote_encrypted_sessions::get_remote_encrypted_session_by_ap_id,
    },
    runner::{self, send_to_inboxes},
};

use super::TaskError;

pub async fn handle_encrypted_note(
    conn: Option<&Db>,
    note: &mut Object,
    inboxes: &mut HashSet<String>,
    sender: Actor,
) -> Option<ApNote> {
    log::debug!("ENCRYPTED NOTE\n{note:#?}");

    async fn do_it(
        conn: Option<&Db>,
        instrument: ApInstrument,
        inboxes: &mut HashSet<String>,
        note: &mut Object,
        sender: Actor,
    ) -> Result<()> {
        if let ApInstrumentType::OlmSession = instrument.kind {
            cfg_if::cfg_if! {
                if #[cfg(feature = "pg")] {
                    let to = serde_json::from_value::<Vec<String>>(note.as_to.clone().unwrap())?;
                } else if #[cfg(feature = "sqlite")] {
                    let to = serde_json::from_str::<Vec<String>>(&note.ap_to.clone())?;
                }
            }
            // save encrypted session
            if let Some((encrypted_session, _olm_session)) =
                get_encrypted_session_by_profile_id_and_ap_to(None, sender.id, to[0].clone()).await
            {
                if let (Some(uuid), Some(hash), Some(content)) = (
                    instrument.clone().uuid,
                    instrument.clone().hash,
                    instrument.clone().content,
                ) {
                    log::debug!("FOUND UUID - UPDATING EXISTING SESSION");
                    if let Some(_session) = update_olm_session(None, uuid, content, hash).await {
                        if let Some(receiver) =
                            runner::actor::get_actor(conn, sender.clone(), to[0].clone()).await
                        {
                            inboxes.insert(receiver.0.as_inbox);
                        }
                    }
                } else {
                    log::debug!("NO UUID - CREATING NEW SESSION");
                    if let Some(_session) =
                        create_olm_session(None, (instrument, encrypted_session.id).into()).await
                    {
                        if let Some(receiver) =
                            runner::actor::get_actor(conn, sender.clone(), to[0].clone()).await
                        {
                            inboxes.insert(receiver.0.as_inbox);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    if let Some(instrument) = &note.ek_instrument {
        cfg_if::cfg_if! {
            if #[cfg(feature = "pg")] {
                let instruments = serde_json::from_value::<ApInstruments>(instrument.clone()).ok()?;
            } else if #[cfg(feature = "sqlite")] {
                let instruments = serde_json::from_str::<ApInstruments>(&instrument.clone()).ok()?;
            }
        }

        match instruments {
            ApInstruments::Multiple(instruments) => {
                for instrument in instruments {
                    let _ = do_it(conn, instrument, inboxes, note, sender.clone()).await;
                }
            }
            ApInstruments::Single(instrument) => {
                let _ = do_it(conn, instrument, inboxes, note, sender).await;
            }
            _ => (),
        }

        Some(note.clone().try_into().ok()?)
    } else {
        log::error!("NO instrument");
        Option::None
    }
}

pub async fn process_join_task(
    conn: Option<Db>,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    log::debug!("RUNNING process_join JOB");

    let conn = conn.as_ref();

    for ap_id in ap_ids {
        let session = get_remote_encrypted_session_by_ap_id(conn, ap_id)
            .await
            .ok_or(TaskError::TaskFailed)?;
        // this is the username of the Enigmatick user who received the Invite
        let identifier =
            get_local_identifier(session.clone().ap_to.clone()).ok_or(TaskError::TaskFailed)?;
        if identifier.kind == LocalIdentifierType::User {
            let username = identifier.identifier;
            let _profile = get_actor_by_username(conn.unwrap(), username.clone())
                .await
                .ok_or(TaskError::TaskFailed)?;
            let session_clone = session.clone();
            let actor = get_actor_by_as_id(conn.unwrap(), session_clone.attributed_to)
                .await
                .map_err(|e| {
                    log::error!("FAILED TO RETRIEVE ACTOR: {e:#?}");
                    TaskError::TaskFailed
                })?;
            log::debug!("ACTOR\n{actor:#?}");
            //let session: ApSession = session.clone().into();

            if let Some(item) = create_processing_item(conn, session.clone().into()).await {
                log::debug!("PROCESSING ITEM\n{item:#?}");
            }
        }
    }

    Ok(())
}

pub async fn send_kexinit_task(
    conn: Option<Db>,
    _channels: Option<EventChannels>,
    uuids: Vec<String>,
) -> Result<(), TaskError> {
    log::debug!("RUNNING send_kexinit JOB");

    let conn = conn.as_ref();

    for uuid in uuids {
        let (encrypted_session, _olm_session) = get_encrypted_session_by_uuid(conn, uuid)
            .await
            .ok_or(TaskError::TaskFailed)?;
        let sender = get_actor(conn.unwrap(), encrypted_session.profile_id)
            .await
            .ok_or(TaskError::TaskFailed)?;

        let mut session: ApSession = encrypted_session.clone().into();
        session.id = Some(format!(
            "{}/session/{}",
            *crate::SERVER_URL,
            encrypted_session.uuid
        ));

        let mut inbox = Option::<String>::None;

        if is_local(session.to.clone().to_string()) {
            if let Some(x) = get_local_identifier(session.to.clone().to_string()) {
                if x.kind == LocalIdentifierType::User {
                    if let Some(profile) = get_actor_by_username(conn.unwrap(), x.identifier).await
                    {
                        inbox = Some(ApActor::from(profile).inbox);
                    }
                }
            }
        } else if let Ok(actor) =
            get_actor_by_as_id(conn.unwrap(), session.to.clone().to_string()).await
        {
            inbox = Some(actor.as_inbox);
        }

        let inbox = inbox.ok_or(TaskError::TaskFailed)?;
        let activity = ApInvite::try_from(session).map_err(|_| TaskError::TaskFailed)?;

        send_to_inboxes(
            conn.unwrap(),
            vec![inbox.clone().into()],
            sender,
            ApActivity::Invite(activity),
        )
        .await
        .map_err(|_| TaskError::TaskFailed)?
    }

    Ok(())
}

pub async fn provide_one_time_key_task(
    conn: Option<Db>,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    log::debug!("RUNNING provide_one_time_key JOB");

    let conn = conn.as_ref();

    for ap_id in ap_ids {
        let session = get_remote_encrypted_session_by_ap_id(conn, ap_id)
            .await
            .ok_or(TaskError::TaskFailed)?;

        let identifier =
            get_local_identifier(session.ap_to.clone()).ok_or(TaskError::TaskFailed)?;
        let actor = get_actor_by_as_id(conn.unwrap(), session.attributed_to.clone())
            .await
            .map_err(|e| {
                log::error!("FAILED TO RETRIEVE ACTOR: {e:#?}");
                TaskError::TaskFailed
            })?;

        if identifier.kind == LocalIdentifierType::User {
            let username = identifier.identifier;
            let profile = get_actor_by_username(conn.unwrap(), username.clone())
                .await
                .ok_or(TaskError::TaskFailed)?;

            let identity_key = profile
                .ek_olm_identity_key
                .clone()
                .ok_or(TaskError::TaskFailed)?;
            let otk = get_olm_one_time_key_by_profile_id(conn, profile.id)
                .await
                .ok_or(TaskError::TaskFailed)?;

            let session = ApSession::from(JoinData {
                one_time_key: otk.key_data,
                identity_key,
                to: session.attributed_to,
                attributed_to: session.ap_to,
                reference: session.ap_id,
            });

            let activity = ApJoin::try_from(session.clone()).map_err(|_| TaskError::TaskFailed)?;

            let encrypted_session: NewEncryptedSession = (session.clone(), profile.id).into();

            if create_encrypted_session(conn, encrypted_session)
                .await
                .is_some()
            {
                match send_to_inboxes(
                    conn.unwrap(),
                    vec![actor.as_inbox.into()],
                    profile,
                    ApActivity::Join(activity),
                )
                .await
                {
                    Ok(_) => {
                        log::info!("JOIN SENT");
                    }
                    Err(e) => {
                        log::error!("ERROR SENDING JOIN: {e:#?}",)
                    }
                }
            } else {
                log::error!("FAILED TO SAVE ENCRYPTED SESSION");
            }
        }
    }

    Ok(())
}
