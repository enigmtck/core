use diesel::prelude::*;
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
        encrypted_sessions::{EncryptedSession, NewEncryptedSession},
        notes::Note,
        olm_one_time_keys::OlmOneTimeKey,
        olm_sessions::{NewOlmSession, OlmSession},
        profiles::Profile,
        remote_encrypted_sessions::RemoteEncryptedSession,
    },
    runner::{
        actor::{get_actor, get_remote_actor_by_ap_id},
        processing::create_processing_item,
        user::{get_profile, get_profile_by_username},
    },
    schema::{encrypted_sessions, olm_one_time_keys, olm_sessions, remote_encrypted_sessions},
};

use super::POOL;

pub fn handle_encrypted_note(
    note: &mut Note,
    inboxes: &mut HashSet<String>,
    sender: Profile,
) -> Option<ApNote> {
    log::debug!("ENCRYPTED NOTE\n{note:#?}");

    fn do_it(
        instrument: ApInstrument,
        inboxes: &mut HashSet<String>,
        note: &mut Note,
        sender: Profile,
    ) {
        let rt = Runtime::new().unwrap();
        let handle = rt.handle();

        if let ApInstrumentType::OlmSession = instrument.kind {
            if let Ok(to) = serde_json::from_value::<Vec<String>>(note.ap_to.clone()) {
                // save encrypted session
                if let Some(encrypted_session) =
                    get_encrypted_session_by_profile_id_and_ap_to(sender.id, to[0].clone())
                {
                    if let (Some(uuid), Some(hash), Some(content)) = (
                        instrument.clone().uuid,
                        instrument.clone().hash,
                        instrument.clone().content,
                    ) {
                        log::debug!("FOUND UUID - UPDATING EXISTING SESSION");
                        if let Some(_session) = update_olm_session(uuid, content, hash) {
                            if let Some(receiver) = handle.block_on(async {
                                get_actor(Some(sender.clone()), to[0].clone()).await
                            }) {
                                inboxes.insert(receiver.0.inbox);
                            }
                        }
                    } else {
                        log::debug!("NO UUID - CREATING NEW SESSION");
                        if let Some(_session) =
                            create_olm_session((instrument, encrypted_session.id).into())
                        {
                            if let Some(receiver) = handle.block_on(async {
                                get_actor(Some(sender.clone()), to[0].clone()).await
                            }) {
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
                        do_it(instrument, inboxes, note, sender.clone());
                    }
                }
                ApInstruments::Single(instrument) => {
                    do_it(instrument, inboxes, note, sender);
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

pub fn create_olm_session(session: NewOlmSession) -> Option<OlmSession> {
    if let Ok(mut conn) = POOL.get() {
        match diesel::insert_into(olm_sessions::table)
            .values(&session)
            .get_result::<OlmSession>(&mut conn)
            .optional()
        {
            Ok(x) => x,
            Err(_) => Option::None,
        }
    } else {
        Option::None
    }
}

pub fn update_olm_session(
    uuid: String,
    session_data: String,
    session_hash: String,
) -> Option<OlmSession> {
    if let Ok(mut conn) = POOL.get() {
        match diesel::update(olm_sessions::table.filter(olm_sessions::uuid.eq(uuid)))
            .set((
                olm_sessions::session_data.eq(session_data),
                olm_sessions::session_hash.eq(session_hash),
            ))
            .get_result::<OlmSession>(&mut conn)
            .optional()
        {
            Ok(x) => x,
            Err(_) => Option::None,
        }
    } else {
        None
    }
}

pub fn process_join(job: Job) -> io::Result<()> {
    let ap_ids = job.args();

    log::debug!("RUNNING process_join JOB");

    for ap_id in ap_ids {
        if let Some(session) =
            get_remote_encrypted_session_by_ap_id(ap_id.as_str().unwrap().to_string())
        {
            // this is the username of the Enigmatick user who received the Invite
            if let Some(identifier) = get_local_identifier(session.ap_to.clone()) {
                if identifier.kind == LocalIdentifierType::User {
                    let username = identifier.identifier;
                    if let Some(_profile) = get_profile_by_username(username.clone()) {
                        if let Some(actor) =
                            get_remote_actor_by_ap_id(session.attributed_to.clone())
                        {
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

        if let Some(encrypted_session) = get_encrypted_session_by_uuid(uuid) {
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
                } else if let Some(actor) =
                    get_remote_actor_by_ap_id(session.to.clone().to_string())
                {
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

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();

        if let Some(session) = get_remote_encrypted_session_by_ap_id(ap_id) {
            if let (Some(identifier), Some(actor)) = (
                get_local_identifier(session.ap_to.clone()),
                get_remote_actor_by_ap_id(session.attributed_to.clone()),
            ) {
                if identifier.kind == LocalIdentifierType::User {
                    let username = identifier.identifier;
                    if let Some(profile) = get_profile_by_username(username.clone()) {
                        if let (Some(identity_key), Some(otk)) = (
                            profile.olm_identity_key.clone(),
                            get_one_time_key(profile.id),
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

                                if create_encrypted_session(encrypted_session).is_some() {
                                    let rt = Runtime::new().unwrap();
                                    tokio::task::block_in_place(|| {
                                        match rt.block_on(async {
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
                                            Err(e) => log::error!("ERROR SENDING JOIN: {e:#?}",),
                                        }
                                    });
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

pub fn create_encrypted_session(
    encrypted_session: NewEncryptedSession,
) -> Option<EncryptedSession> {
    if let Ok(mut conn) = POOL.get() {
        match diesel::insert_into(encrypted_sessions::table)
            .values(&encrypted_session)
            .get_result::<EncryptedSession>(&mut conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn get_encrypted_session_by_profile_id_and_ap_to(
    profile_id: i32,
    ap_to: String,
) -> Option<EncryptedSession> {
    if let Ok(mut conn) = POOL.get() {
        match encrypted_sessions::table
            .filter(encrypted_sessions::profile_id.eq(profile_id))
            .filter(encrypted_sessions::ap_to.eq(ap_to))
            .order(encrypted_sessions::updated_at.desc())
            .first::<EncryptedSession>(&mut conn)
        {
            Ok(x) => Option::from(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn get_encrypted_session_by_uuid(uuid: String) -> Option<EncryptedSession> {
    log::debug!("looking for encrypted_session_by_uuid: {:#?}", uuid);
    if let Ok(mut conn) = POOL.get() {
        match encrypted_sessions::table
            .filter(encrypted_sessions::uuid.eq(uuid))
            .first::<EncryptedSession>(&mut conn)
        {
            Ok(x) => Option::from(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn get_one_time_key(profile_id: i32) -> Option<OlmOneTimeKey> {
    log::debug!("IN get_one_time_key");
    if let Ok(mut conn) = POOL.get() {
        if let Ok(Some(otk)) = olm_one_time_keys::table
            .filter(olm_one_time_keys::profile_id.eq(profile_id))
            .filter(olm_one_time_keys::distributed.eq(false))
            .first::<OlmOneTimeKey>(&mut conn)
            .optional()
        {
            log::debug!("OTK\n{otk:#?}");
            match diesel::update(olm_one_time_keys::table.find(otk.id))
                .set(olm_one_time_keys::distributed.eq(true))
                .get_results::<OlmOneTimeKey>(&mut conn)
            {
                Ok(mut x) => x.pop(),
                Err(e) => {
                    log::error!("FAILED TO RETRIEVE OTK: {e:#?}");
                    Option::None
                }
            }
        } else {
            Option::None
        }
    } else {
        Option::None
    }
}

pub fn get_remote_encrypted_session_by_ap_id(apid: String) -> Option<RemoteEncryptedSession> {
    log::debug!("looking for remote_encrypted_session_by_ap_id: {:#?}", apid);
    if let Ok(mut conn) = POOL.get() {
        match remote_encrypted_sessions::table
            .filter(remote_encrypted_sessions::ap_id.eq(apid))
            .first::<RemoteEncryptedSession>(&mut conn)
        {
            Ok(x) => Option::from(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}
