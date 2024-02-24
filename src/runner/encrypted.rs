use std::collections::HashSet;
use std::io;

use faktory::Job;
use tokio::runtime::Runtime;

use crate::{
    activity_pub::{
        sender::send_activity, ApActivity, ApActor, ApInstrument, ApInstrumentType, ApInstruments,
        ApInvite, ApJoin, ApNote, ApSession, JoinData,
    },
    helper::{get_local_identifier, is_local, LocalIdentifierType},
    models::{
        encrypted_sessions::{
            create_encrypted_session, get_encrypted_session_by_profile_id_and_ap_to,
            get_encrypted_session_by_uuid, NewEncryptedSession,
        },
        notes::Note,
        olm_one_time_keys::get_olm_one_time_key_by_profile_id,
        olm_sessions::{create_olm_session, update_olm_session},
        profiles::Profile,
        remote_actors::get_remote_actor_by_ap_id,
        remote_encrypted_sessions::get_remote_encrypted_session_by_ap_id,
    },
    runner::{
        actor::get_actor,
        processing::create_processing_item,
        user::{get_profile, get_profile_by_username},
    },
    POOL,
};

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
                    get_encrypted_session_by_profile_id_and_ap_to(
                        POOL.get()
                            .expect("unable to get database connection")
                            .into(),
                        sender.id,
                        to[0].clone(),
                    )
                    .await
                {
                    if let (Some(uuid), Some(hash), Some(content)) = (
                        instrument.clone().uuid,
                        instrument.clone().hash,
                        instrument.clone().content,
                    ) {
                        log::debug!("FOUND UUID - UPDATING EXISTING SESSION");
                        if let Some(_session) = update_olm_session(
                            POOL.get()
                                .expect("failed to get database connection")
                                .into(),
                            uuid,
                            content,
                            hash,
                        )
                        .await
                        {
                            if let Some(receiver) = get_actor(sender.clone(), to[0].clone()).await {
                                inboxes.insert(receiver.0.inbox);
                            }
                        }
                    } else {
                        log::debug!("NO UUID - CREATING NEW SESSION");
                        if let Some(_session) = create_olm_session(
                            POOL.get()
                                .expect("failed to get database connection")
                                .into(),
                            (instrument, encrypted_session.id).into(),
                        )
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

pub fn process_join(job: Job) -> io::Result<()> {
    let ap_ids = job.args();

    log::debug!("RUNNING process_join JOB");
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for ap_id in ap_ids {
        if let Some(session) = handle.block_on(async {
            get_remote_encrypted_session_by_ap_id(
                POOL.get()
                    .expect("failed to get database connection")
                    .into(),
                ap_id.as_str().unwrap().to_string(),
            )
            .await
        }) {
            // this is the username of the Enigmatick user who received the Invite
            if let Some(identifier) = get_local_identifier(session.clone().ap_to.clone()) {
                if identifier.kind == LocalIdentifierType::User {
                    let username = identifier.identifier;
                    if let Some(_profile) = get_profile_by_username(username.clone()) {
                        let session_clone = session.clone();
                        if let Ok(actor) = handle.block_on(async move {
                            get_remote_actor_by_ap_id(
                                POOL.get()
                                    .expect("failed to get database connection")
                                    .into(),
                                session_clone.attributed_to,
                            )
                            .await
                        }) {
                            log::debug!("ACTOR\n{actor:#?}");
                            //let session: ApSession = session.clone().into();

                            if let Some(item) = create_processing_item(session.clone().into()) {
                                log::debug!("PROCESSING ITEM\n{item:#?}");
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn send_kexinit(job: Job) -> io::Result<()> {
    log::debug!("RUNNING send_kexinit JOB");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();

        if let Some((encrypted_session, _olm_session)) = handle.block_on(async {
            get_encrypted_session_by_uuid(
                POOL.get()
                    .expect("failed to get database connection")
                    .into(),
                uuid,
            )
            .await
        }) {
            if let Some(sender) = get_profile(encrypted_session.profile_id) {
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
                            if let Some(profile) = get_profile_by_username(x.identifier) {
                                inbox = Option::from(ApActor::from(profile).inbox);
                            }
                        }
                    }
                } else if let Ok(actor) = handle.block_on(async {
                    get_remote_actor_by_ap_id(
                        POOL.get()
                            .expect("failed to get database connection")
                            .into(),
                        session.to.clone().to_string(),
                    )
                    .await
                }) {
                    inbox = Option::from(actor.inbox);
                }

                if let Some(inbox) = inbox {
                    if let Ok(activity) = ApInvite::try_from(session) {
                        handle.block_on(async {
                            match send_activity(ApActivity::Invite(activity), sender, inbox.clone())
                                .await
                            {
                                Ok(_) => {
                                    log::info!("INVITE SENT: {inbox:#?}");
                                }
                                Err(e) => log::error!("error: {:#?}", e),
                            }
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

// ChatGPT generated some of this function; it looks good from a cursory
// overview but the original is below for reference.
pub fn provide_one_time_key(job: Job) -> io::Result<()> {
    log::debug!("RUNNING provide_one_time_key JOB");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();

        if let Some(session) = handle.block_on(async {
            get_remote_encrypted_session_by_ap_id(
                POOL.get()
                    .expect("failed to get database connection")
                    .into(),
                ap_id,
            )
            .await
        }) {
            if let (Some(identifier), Ok(actor)) = (
                get_local_identifier(session.ap_to.clone()),
                handle.block_on(async {
                    get_remote_actor_by_ap_id(
                        POOL.get()
                            .expect("failed to get database connection")
                            .into(),
                        session.attributed_to.clone(),
                    )
                    .await
                }),
            ) {
                if identifier.kind == LocalIdentifierType::User {
                    let username = identifier.identifier;
                    if let Some(profile) = get_profile_by_username(username.clone()) {
                        if let (Some(identity_key), Some(otk)) = (
                            profile.olm_identity_key.clone(),
                            handle.block_on(async {
                                get_olm_one_time_key_by_profile_id(
                                    POOL.get()
                                        .expect("failed to get database connection")
                                        .into(),
                                    profile.id,
                                )
                                .await
                            }),
                        ) {
                            let session = ApSession::from(JoinData {
                                one_time_key: otk.key_data,
                                identity_key,
                                to: session.attributed_to,
                                attributed_to: session.ap_to,
                                reference: session.ap_id,
                            });

                            if let Ok(activity) = ApJoin::try_from(session.clone()) {
                                let encrypted_session: NewEncryptedSession =
                                    (session.clone(), profile.id).into();

                                if handle.block_on(async {
                                    create_encrypted_session(
                                        POOL.get()
                                            .expect("failed to get database connection")
                                            .into(),
                                        encrypted_session,
                                    )
                                    .await
                                    .is_some()
                                }) {
                                    match handle.block_on(async {
                                        send_activity(
                                            ApActivity::Join(activity),
                                            profile,
                                            actor.inbox,
                                        )
                                        .await
                                    }) {
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
                    }
                }
            }
        }
    }

    Ok(())
}

// fn provide_one_time_key(job: Job) -> io::Result<()> {
//     log::debug!("RUNNING provide_one_time_key JOB");

//     // look up remote_encrypted_session with ap_id from job.args()

//     let rt = Runtime::new().unwrap();
//     let handle = rt.handle();

//     for ap_id in job.args() {
//         let ap_id = ap_id.as_str().unwrap().to_string();

//         if let Some(session) = get_remote_encrypted_session_by_ap_id(ap_id) {
//             // this is the username of the Enigmatick user who received the Invite
//             log::debug!("SESSION\n{session:#?}");
//             if let Some(username) = get_local_username_from_ap_id(session.ap_to.clone()) {
//                 log::debug!("USERNAME: {username}");
//                 if let Some(profile) = get_profile_by_username(username.clone()) {
//                     log::debug!("PROFILE\n{profile:#?}");
//                     if let Some(actor) = get_remote_actor_by_ap_id(session.attributed_to.clone()) {
//                         log::debug!("ACTOR\n{actor:#?}");
//                         // send Join activity with Identity and OTK to attributed_to

//                         if let Some(identity_key) = profile.olm_identity_key.clone() {
//                             log::debug!("IDENTITY KEY: {identity_key}");
//                             if let Some(otk) = get_one_time_key(profile.id) {
//                                 log::debug!("IDK\n{identity_key:#?}");
//                                 log::debug!("OTK\n{otk:#?}");

//                                 let session = ApSession::from(JoinData {
//                                     one_time_key: otk.key_data,
//                                     identity_key,
//                                     to: session.attributed_to,
//                                     attributed_to: session.ap_to,
//                                     reference: session.ap_id,
//                                 });

//                                 let activity = ApActivity::from(session.clone());
//                                 let encrypted_session: NewEncryptedSession =
//                                     (session.clone(), profile.id).into();

//                                 // this activity should be saved so that the id makes sense
//                                 // but it's not right now
//                                 log::debug!("JOIN ACTIVITY\n{activity:#?}");

//                                 if create_encrypted_session(encrypted_session).is_some() {
//                                     handle.block_on(async {
//                                         match send_activity(activity, profile, actor.inbox).await {
//                                             Ok(_) => {
//                                                 info!("JOIN SENT");
//                                             }
//                                             Err(e) => error!("ERROR SENDING JOIN: {e:#?}"),
//                                         }
//                                     });
//                                 } else {
//                                     log::error!("FAILED TO SAVE ENCRYPTED SESSION");
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//     }

//     Ok(())
// }
