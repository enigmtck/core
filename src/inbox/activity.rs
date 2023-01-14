use crate::{
    FaktoryConnection,
    activity_pub::{ApActivity, ApObject, ApActivityType, ApInstrument, ApBasicContentType, ApSession},
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
        encrypted_sessions::NewEncryptedSession,
    }, assign_to_faktory,
};
use log::debug;
use rocket::http::Status;

pub async fn delete(conn: Db, activity: ApActivity) -> Result<Status, Status> {
    if let ApObject::Plain(ap_id) = activity.object {
        if ap_id == activity.actor && delete_remote_actor_by_ap_id(&conn, ap_id).await.is_ok() {
            debug!("remote actor record deleted");
        }
    }
 
    Ok(Status::Accepted)
}

pub async fn create(conn: Db, faktory: FaktoryConnection, activity: ApActivity, profile: Profile) -> Result<Status, Status> {
    match activity.object {
        ApObject::Note(x) => {
            let n = NewRemoteNote::from((x, profile.id));

            if let Some(created_note) = create_remote_note(&conn, n).await {
                log::debug!("created_remote_note\n{:#?}", created_note);
                match assign_to_faktory(faktory,
                                        String::from("process_remote_note"),
                                        vec![created_note.ap_id]) {
                    Ok(_) => Ok(Status::Accepted),
                    Err(_) => Err(Status::NoContent)
                }
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

        match assign_to_faktory(faktory,
                                String::from("acknowledge_followers"),
                                vec![created_follower.uuid]) {
            Ok(_) => Ok(Status::Accepted),
            Err(_) => Err(Status::NoContent)
        }
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

pub async fn save_identity_key(conn: &Db, activity: ApActivity, profile: Profile) {
    async fn extract(conn: &Db, instrument: ApObject, profile: Profile, session: ApSession) {
        if let ApObject::Basic(basic) = instrument {
            if basic.kind == ApBasicContentType::IdentityKey {
                let mut keystore = profile.keystore.clone();
                if let Some(mut keys) = keystore.olm_external_identity_keys {
                    keys.insert(session.attributed_to, basic.content);
                    keystore.olm_external_identity_keys = Some(keys);
                    update_olm_external_identity_keys_by_username(conn,
                                                                  profile.username,
                                                                  keystore).await;
                }
            }
        }
    }
    
    // make sure we have the IdentityKey for the sender recorded in the local user's Profile
    if let ApObject::Session(session) = activity.object {
        match session.clone().instrument {
            ApInstrument::Single(instrument) => {
                extract(conn, *instrument, profile, session).await;
            },
            ApInstrument::Multiple(instruments) => {
                for instrument in instruments {
                    extract(conn, instrument, profile.clone(), session.clone()).await;
                }
            },
            _ => ()
        }
    }
}

pub async fn invite(conn: Db, faktory: FaktoryConnection, activity: ApActivity, profile: Profile)
                    -> Result<Status, Status>
{
    save_identity_key(&conn, activity.clone(), profile.clone()).await;
    
    if let Some(session) =
        create_remote_encrypted_session(
            &conn,
            (activity.clone(), profile.id).into()
        ).await
    {
        match assign_to_faktory(faktory,
                                String::from("provide_one_time_key"),
                                vec![session.ap_id]) {
            Ok(_) => Ok(Status::Accepted),
            Err(_) => Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

pub async fn join(conn: Db, faktory: FaktoryConnection, activity: ApActivity, profile: Profile)
                    -> Result<Status, Status>
{
    if create_remote_encrypted_session(
        &conn,
        (activity.clone(), profile.id).into()
    ).await.is_some() {
        if let ApObject::Session(session) = activity.object {

            if let Some(ap_id) = session.id {
                match assign_to_faktory(faktory,
                                        String::from("process_join"),
                                        vec![ap_id]) {
                    Ok(_) => Ok(Status::Accepted),
                    Err(_) => Err(Status::NoContent)
                }
            } else {
                log::error!("no id found on ApSession (remote_encrypted_session) object");
                Err(Status::NoContent)
            }
            // // this is the inbox; we shouldn't be creating the encrypted_session here;
            // // that's an outbox activity to generate the UUID that will be used by remote
            // // servers for locally-created assets.
            // if create_encrypted_session(
            //     &conn,
            //     (session, profile.id).into()
            // ).await.is_some() {
            //     Ok(Status::Accepted)
            // } else {
            //     Err(Status::NoContent)
            // }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}
