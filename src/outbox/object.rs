use std::collections::HashSet;

use crate::{
    activity_pub::{retriever, sender, ApActivity, ApActor, ApNote, ApSession},
    db::{
        create_encrypted_session, create_note, get_followers_by_profile_id,
        get_profile_by_username, get_remote_actor_by_ap_id, Db,
    },
    helper::{get_local_username_from_ap_id, is_local, is_public},
    models::{encrypted_sessions::NewEncryptedSession, notes::NewNote, profiles::Profile},
    signing::{sign, Method, SignParams},
};
use log::debug;
use reqwest::Client;
use rocket::http::Status;

async fn get_follower_inboxes(conn: &Db, profile: Profile) -> HashSet<String> {
    let mut inboxes: HashSet<String> = HashSet::new();

    for follower in get_followers_by_profile_id(conn, profile.id).await {
        if let Some(actor) = get_remote_actor_by_ap_id(conn, follower.actor).await {
            inboxes.insert(actor.inbox);
        }
    }

    inboxes
}

pub async fn note(conn: Db, note: ApNote, profile: Profile) -> Result<Status, Status> {
    // ApNote -> NewNote -> ApNote -> ApActivity
    // UUID is set in NewNote
    let mut n = NewNote::from((note.clone(), profile.id));

    let mut inboxes: HashSet<String> = HashSet::new();

    for recipient in note.to {
        let profile = profile.clone();

        if is_public(recipient.clone()) {
            inboxes.extend(get_follower_inboxes(&conn, profile.clone()).await);
            if let Some(cc) = n.clone().cc {
                let mut cc: Vec<String> = serde_json::from_value(cc).unwrap();
                cc.push(ApActor::from(profile).followers);
                n.cc = Option::from(serde_json::to_value(cc).unwrap());
            } else {
                n.cc = Option::from(
                    serde_json::to_value(vec![ApActor::from(profile).followers]).unwrap(),
                );
            }
        } else if is_local(recipient.clone()) {
            if let Some(username) = get_local_username_from_ap_id(recipient.to_string()) {
                if let Some(profile) = get_profile_by_username(&conn, username).await {
                    inboxes.insert(ApActor::from(profile).inbox);
                }
            }
        } else if let Some(receiver) =
            retriever::get_actor(&conn, profile.clone(), recipient.clone()).await
        {
            inboxes.insert(receiver.0.inbox);
        }
    }

    if create_note(&conn, n.clone()).await.is_some() {
        let create = ApActivity::from(ApNote::from(n.clone()));
        log::debug!("inboxes: {inboxes:#?}");
        log::debug!("create: {create:#?}");

        for url in inboxes {
            let body = Option::from(serde_json::to_string(&create).unwrap());
            let method = Method::Post;

            let signature = sign(SignParams {
                profile: profile.clone(),
                url: url.clone(),
                body: body.clone(),
                method,
            });

            let client = Client::new()
                .post(&url)
                .header("Date", signature.date)
                .header("Digest", signature.digest.unwrap())
                .header("Signature", &signature.signature)
                .header(
                    "Content-Type",
                    "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"",
                )
                .body(body.unwrap());

            if let Ok(resp) = client.send().await {
                if let Ok(text) = resp.text().await {
                    debug!("send successful to: {}\n{}", url, text);
                }
            }
        }

        Ok(Status::Accepted)
    } else {
        Err(Status::NoContent)
    }
}

pub async fn session(conn: Db, session: ApSession, profile: Profile) -> Result<Status, Status> {
    let encrypted_session: NewEncryptedSession = (session.clone(), profile.id).into();

    if create_encrypted_session(&conn, encrypted_session.clone())
        .await
        .is_some()
    {
        let mut session = session;
        session.id = Option::from(format!(
            "{}/encrypted-sessions/{}",
            *crate::SERVER_URL,
            encrypted_session.uuid
        ));

        let mut inbox = Option::<String>::None;

        if is_local(session.to.clone()) {
            if let Some(username) = get_local_username_from_ap_id(session.to.clone()) {
                if let Some(profile) = get_profile_by_username(&conn, username).await {
                    inbox = Option::from(ApActor::from(profile).inbox);
                }
            }
        } else if let Some(actor) = get_remote_actor_by_ap_id(&conn, session.to.clone()).await {
            inbox = Option::from(actor.inbox);
        }

        if let Some(inbox) = inbox {
            let activity = ApActivity::from(session);
            if sender::send_activity(activity, profile, inbox)
                .await
                .is_ok()
            {
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
