use crate::{
    activity_pub::{ApActivity, ApActor, sender, retriever, ApNote, ApSession},
    db::{
        get_remote_actor_by_ap_id,
        create_note,
        create_encrypted_session,
        get_profile_by_username,
        Db
    },
    helper::{
        get_local_username_from_ap_id,
        is_local
    },
    models::{
        profiles::Profile,
        notes::NewNote,
        encrypted_sessions::NewEncryptedSession,
    },
    signing::{Method, sign, SignParams},
};
use log::debug;
use rocket::http::Status;
use reqwest::Client;

pub async fn note(conn: Db, note: ApNote, profile: Profile) -> Result<Status, Status> {
    // ApNote -> NewNote -> ApNote -> ApActivity
    // UUID is set in NewNote
    let n = NewNote::from((note.clone(), profile.id));
    let create = ApActivity::from(ApNote::from(n.clone()));
    
    if create_note(&conn, n).await.is_some() {
        for recipient in note.to {                            
            let profile = profile.clone();

            let mut inbox = Option::<String>::None;
            
            if is_local(recipient.clone()) {
                if let Some(username) = get_local_username_from_ap_id(
                    recipient.to_string()) {
                    if let Some(profile) =
                        get_profile_by_username(&conn, username).await {
                            inbox = Option::from(ApActor::from(profile).inbox);
                        }
                }
            } else if let Some(receiver) = retriever::get_actor(&conn,
                                                         profile.clone(),
                                                                recipient.clone()).await {
                inbox = Option::from(receiver.inbox);
            }

            if let Some(inbox) = inbox {
                let url = inbox; 

                let body = Option::from(serde_json::to_string(&create).unwrap());
                let method = Method::Post;
                
                let signature = sign(
                    SignParams { profile,
                                 url: url.clone(),
                                 body: body.clone(),
                                 method }
                );

                let client = Client::new().post(&url)
                    .header("Date", signature.date)
                    .header("Digest", signature.digest.unwrap())
                    .header("Signature", &signature.signature)
                    .header("Content-Type",
                            "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"")
                    .body(body.unwrap());
                
                if let Ok(resp) = client.send().await {
                    if let Ok(text) = resp.text().await {
                        debug!("send successful to: {}\n{}", recipient, text);
                    }
                }
            }
        }
        Ok(Status::Accepted)
    } else {
        Err(Status::NoContent)
    }
}

pub async fn session(conn: Db, session: ApSession, profile: Profile) -> Result<Status, Status> {
    let mut encrypted_session = NewEncryptedSession::from(session.clone());
    encrypted_session.profile_id = profile.id;

    if create_encrypted_session(&conn, encrypted_session.clone()).await.is_some() {
        let mut session = session;
        session.id = Option::from(format!("{}/encrypted-sessions/{}",
                                          *crate::SERVER_URL,
                                          encrypted_session.uuid));

        let mut inbox = Option::<String>::None;
        
        if is_local(session.to.clone()) {
            if let Some(username) = get_local_username_from_ap_id(
                session.to.clone()) {
                if let Some(profile) =
                    get_profile_by_username(&conn, username).await {
                        inbox = Option::from(ApActor::from(profile).inbox);
                    }
            }
        } else if let Some(actor) = get_remote_actor_by_ap_id(
            &conn,
            session.to.clone()).await {
            inbox = Option::from(actor.inbox);
        }

        if let Some(inbox) = inbox {
            let activity = ApActivity::from(session);
            if sender::send_activity(activity, profile, inbox).await.is_ok() {
                debug!("sent invite request successfully");
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

