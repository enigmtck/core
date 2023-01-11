use crate::{
    FaktoryConnection,
    activity_pub::{ApActivity, ApObject, ApActivityType, ApInstrument, ApBasicContentType},
    db::{
        delete_remote_actor_by_ap_id,
        create_remote_note,
        create_follower,
        get_remote_activity_by_ap_id,
        delete_follower_by_ap_id,
        update_leader_by_uuid,
        create_remote_encrypted_session,
        create_encrypted_session,
        Db,
    },
    models::{
        profiles::{Profile, update_olm_external_identity_keys_by_username},
        remote_notes::NewRemoteNote,
        followers::NewFollower,
        remote_encrypted_sessions::NewRemoteEncryptedSession,
        encrypted_sessions::{EncryptedSession, NewEncryptedSession},
    },
};
use log::debug;
use rocket::http::Status;
use faktory::Job;

pub async fn delete(conn: Db, activity: ApActivity) -> Result<Status, Status> {
    if let ApObject::Plain(ap_id) = activity.object {
        if ap_id == activity.actor && delete_remote_actor_by_ap_id(&conn, ap_id).await.is_ok() {
            debug!("remote actor record deleted");
        }
    }
 
    Ok(Status::Accepted)
}

pub async fn create(conn: Db, activity: ApActivity, profile: Profile) -> Result<Status, Status> {
    match activity.object {
        ApObject::Note(x) => {
            let mut n = NewRemoteNote::from(x);
            n.profile_id = profile.id;

            if let Some(created_note) = create_remote_note(&conn, n). await {
                log::debug!("created_remote_note\n{:#?}", created_note);
                Ok(Status::Accepted)
            } else {
                log::debug!("create_remote_note failed");
                Err(Status::NoContent)
            }
        },
        _ => Err(Status::NoContent)
    }
}

pub async fn follow(conn: Db, faktory: FaktoryConnection, activity: ApActivity, profile: Profile) -> Result<Status, Status> {
    let mut f = NewFollower::from(activity);
    f.profile_id = profile.id;

    if let Some(created_follower) = create_follower(&conn, f). await {
        log::debug!("created_follower\n{:#?}", created_follower);

        match faktory.producer.try_lock() {
            Ok(mut x) => {
                if x.enqueue(Job::new("acknowledge_followers", vec![created_follower.uuid]))
                    .is_err() {
                        log::error!("failed to enqueue job");
                    }
            },
            Err(e) => log::debug!("failed to lock mutex: {}", e)
        }
        
        Ok(Status::Accepted)
    } else {
        log::debug!("create_follower failed");
        Err(Status::NoContent)
    }
}

pub async fn undo(conn: Db, activity: ApActivity) -> Result<Status, Status> {
    if let ApObject::Identifier(x) = activity.object {
        if let Some(x) = get_remote_activity_by_ap_id(&conn, x.id).await {
            if x.kind == ApActivityType::Follow.to_string() &&
                delete_follower_by_ap_id(&conn, x.ap_id).await {
                    Ok(Status::Accepted)
                } else {
                    Err(Status::NoContent)
                }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

pub async fn accept(conn: Db, activity: ApActivity) -> Result<Status, Status> {
    if let ApObject::Identifier(x) = activity.object {
        let ap_id_re = regex::Regex::new(r#"(\w+://)(.+?/)+(.+)"#).unwrap();
        if let Some(ap_id_match) = ap_id_re.captures(&x.id) {
            debug!("ap_id_match: {:#?}", ap_id_match);

            let matches = ap_id_match.len();
            let uuid = ap_id_match.get(matches-1).unwrap().as_str();

            if let Some(id) = activity.id {
                if update_leader_by_uuid(&conn, uuid.to_string(), id).await.is_some() {
                    Ok(Status::Accepted)
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

pub async fn invite(conn: Db, faktory: FaktoryConnection, activity: ApActivity, profile: Profile)
                    -> Result<Status, Status>
{
    if let ApObject::Session(session) = activity.clone().object {
        if let ApInstrument::Single(instrument) = session.instrument {
            if let ApObject::Basic(basic) = *instrument {
                if basic.kind == ApBasicContentType::IdentityKey {
                    let mut keystore = profile.keystore.clone();
                    if let Some(mut keys) = keystore.olm_external_identity_keys {
                        keys.insert(session.attributed_to, basic.content);
                        keystore.olm_external_identity_keys = Some(keys);
                        update_olm_external_identity_keys_by_username(&conn,
                                                                      profile.username,
                                                                      keystore).await;
                    }
                }
            }
        }
    }
    
    let mut remote_encrypted_session = NewRemoteEncryptedSession::from(activity.clone());
    remote_encrypted_session.profile_id = profile.id;

    let ap_id = remote_encrypted_session.ap_id.clone();
    
    if create_remote_encrypted_session(&conn, remote_encrypted_session)
        .await.is_some()
    {

        if let Ok(mut producer) = faktory.producer.try_lock() {
            if producer.enqueue(
                Job::new("provide_one_time_key", vec![ap_id])).is_err() {
                log::error!("failed to enqueue job");
            }
        } else {
            log::debug!("failed to lock mutex");
        }
        
        Ok(Status::Accepted)
    } else {
        Err(Status::NoContent)
    }
}

pub async fn join(conn: Db, activity: ApActivity, profile: Profile)
                    -> Result<Status, Status>
{
    let mut remote_encrypted_session =
        NewRemoteEncryptedSession::from(activity.clone());
    remote_encrypted_session.profile_id = profile.id;
    
    if create_remote_encrypted_session(&conn, remote_encrypted_session).await.is_some() {

        if let ApObject::Session(session) = activity.object {
            let mut encrypted_session: NewEncryptedSession = session.into();
            encrypted_session.profile_id = profile.id;
            
            if create_encrypted_session(&conn, encrypted_session).await.is_some() {
                Ok(Status::Accepted)
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}
