use std::collections::HashSet;
use std::io;

use faktory::Job;
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{
        sender::send_activity, ApActivity, ApActor, ApInstrument, ApInstrumentType, ApInstruments,
        ApInvite, ApJoin, ApNote, ApSession, JoinData,
    },
    db::Db,
    helper::{get_local_identifier, is_local, LocalIdentifierType},
    models::{
        encrypted_sessions::{
            create_encrypted_session, get_encrypted_session_by_profile_id_and_ap_to,
            get_encrypted_session_by_uuid, NewEncryptedSession,
        },
        notes::Note,
        olm_one_time_keys::get_olm_one_time_key_by_profile_id,
        olm_sessions::{create_olm_session, update_olm_session},
        processing_queue::create_processing_item,
        profiles::{get_profile, get_profile_by_username, Profile},
        remote_actors::get_remote_actor_by_ap_id,
        remote_encrypted_sessions::get_remote_encrypted_session_by_ap_id,
    },
    runner::actor::get_actor,
};

use super::TaskError;

pub async fn handle_encrypted_note(
    note: &mut Note,
    inboxes: &mut HashSet<String>,
    sender: Profile,
) -> Option<ApNote> {
    log::debug!("ENCRYPTED NOTE\n{note:#?}");

    async fn do_it(
        instrument: ApInstrument,
        inboxes: &mut HashSet<String>,
        note: &mut Note,
        sender: Profile,
    ) {
        if let ApInstrumentType::OlmSession = instrument.kind {
            if let Ok(to) = serde_json::from_value::<Vec<String>>(note.ap_to.clone()) {
                // save encrypted session
                if let Some((encrypted_session, _olm_session)) =
                    get_encrypted_session_by_profile_id_and_ap_to(None, sender.id, to[0].clone())
                        .await
                {
                    if let (Some(uuid), Some(hash), Some(content)) = (
                        instrument.clone().uuid,
                        instrument.clone().hash,
                        instrument.clone().content,
                    ) {
                        log::debug!("FOUND UUID - UPDATING EXISTING SESSION");
                        if let Some(_session) = update_olm_session(None, uuid, content, hash).await
                        {
                            if let Some(receiver) = get_actor(sender.clone(), to[0].clone()).await {
                                inboxes.insert(receiver.0.inbox);
                            }
                        }
                    } else {
                        log::debug!("NO UUID - CREATING NEW SESSION");
                        if let Some(_session) =
                            create_olm_session(None, (instrument, encrypted_session.id).into())
                                .await
                        {
                            if let Some(receiver) = get_actor(sender.clone(), to[0].clone()).await {
                                inboxes.insert(receiver.0.inbox);
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(instrument) = &note.instrument {
        if let Ok(instruments) = serde_json::from_value::<ApInstruments>(instrument.clone()) {
            match instruments {
                ApInstruments::Multiple(instruments) => {
                    for instrument in instruments {
                        do_it(instrument, inboxes, note, sender.clone()).await;
                    }
                }
                ApInstruments::Single(instrument) => {
                    do_it(instrument, inboxes, note, sender).await;
                }
                _ => (),
            }

            Some(note.clone().into())
        } else {
            log::error!("INVALID INSTRUMENT\n{instrument:#?}");
            Option::None
        }
    } else {
        log::error!("NO instrument");
        Option::None
    }
}

pub async fn process_join_task(conn: Option<Db>, ap_ids: Vec<String>) -> Result<(), TaskError> {
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
            let _profile = get_profile_by_username(conn, username.clone())
                .await
                .ok_or(TaskError::TaskFailed)?;
            let session_clone = session.clone();
            let actor = get_remote_actor_by_ap_id(conn, session_clone.attributed_to)
                .await
                .map_err(|_| TaskError::TaskFailed)?;
            log::debug!("ACTOR\n{actor:#?}");
            //let session: ApSession = session.clone().into();

            if let Some(item) = create_processing_item(conn, session.clone().into()).await {
                log::debug!("PROCESSING ITEM\n{item:#?}");
            }
        }
    }

    Ok(())
}

pub fn process_join(job: Job) -> io::Result<()> {
    log::debug!("RUNNING process_join JOB");
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    handle
        .block_on(async {
            process_join_task(None, serde_json::from_value(job.args().into()).unwrap()).await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub async fn send_kexinit_task(conn: Option<Db>, uuids: Vec<String>) -> Result<(), TaskError> {
    log::debug!("RUNNING send_kexinit JOB");

    let conn = conn.as_ref();

    for uuid in uuids {
        let (encrypted_session, _olm_session) = get_encrypted_session_by_uuid(conn, uuid)
            .await
            .ok_or(TaskError::TaskFailed)?;
        let sender = get_profile(conn, encrypted_session.profile_id)
            .await
            .ok_or(TaskError::TaskFailed)?;

        let mut session: ApSession = encrypted_session.clone().into();
        session.id = Option::from(format!(
            "{}/session/{}",
            *crate::SERVER_URL,
            encrypted_session.uuid
        ));

        let mut inbox = Option::<String>::None;

        if is_local(session.to.clone().to_string()) {
            if let Some(x) = get_local_identifier(session.to.clone().to_string()) {
                if x.kind == LocalIdentifierType::User {
                    if let Some(profile) = get_profile_by_username(conn, x.identifier).await {
                        inbox = Option::from(ApActor::from(profile).inbox);
                    }
                }
            }
        } else if let Ok(actor) =
            get_remote_actor_by_ap_id(conn, session.to.clone().to_string()).await
        {
            inbox = Option::from(actor.inbox);
        }

        let inbox = inbox.ok_or(TaskError::TaskFailed)?;
        let activity = ApInvite::try_from(session).map_err(|_| TaskError::TaskFailed)?;

        send_activity(ApActivity::Invite(activity), sender, inbox.clone())
            .await
            .map_err(|_| TaskError::TaskFailed)?
    }

    Ok(())
}

pub fn send_kexinit(job: Job) -> io::Result<()> {
    log::debug!("RUNNING send_kexinit JOB");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    handle
        .block_on(async {
            send_kexinit_task(None, serde_json::from_value(job.args().into()).unwrap()).await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub fn provide_one_time_key(job: Job) -> io::Result<()> {
    log::debug!("RUNNING provide_one_time_key JOB");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    handle
        .block_on(async {
            provide_one_time_key_task(None, serde_json::from_value(job.args().into()).unwrap())
                .await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub async fn provide_one_time_key_task(
    conn: Option<Db>,
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
        let actor = get_remote_actor_by_ap_id(conn, session.attributed_to.clone())
            .await
            .map_err(|_| TaskError::TaskFailed)?;

        if identifier.kind == LocalIdentifierType::User {
            let username = identifier.identifier;
            let profile = get_profile_by_username(conn, username.clone())
                .await
                .ok_or(TaskError::TaskFailed)?;

            let identity_key = profile
                .olm_identity_key
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
                match send_activity(ApActivity::Join(activity), profile, actor.inbox).await {
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
