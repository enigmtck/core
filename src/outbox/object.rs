use std::collections::HashSet;

use crate::{
    activity_pub::{retriever, sender, ApActivity, ApActor, ApNote, ApSession},
    db::{
        create_encrypted_session, create_note, get_followers_by_profile_id,
        get_profile_by_username, get_remote_actor_by_ap_id, Db,
    },
    fairings::{
        events::EventChannels,
        faktory::{assign_to_faktory, FaktoryConnection},
    },
    helper::{get_local_username_from_ap_id, is_local, is_public},
    models::{encrypted_sessions::NewEncryptedSession, notes::NewNote, profiles::Profile},
    //    signing::{sign, Method, SignParams},
};
use log::debug;
//use reqwest::Client;
use rocket::http::Status;

// async fn get_follower_inboxes(conn: &Db, profile: Profile) -> HashSet<String> {
//     let mut inboxes: HashSet<String> = HashSet::new();

//     for follower in get_followers_by_profile_id(conn, profile.id).await {
//         if let Some(actor) = get_remote_actor_by_ap_id(conn, follower.actor).await {
//             inboxes.insert(actor.inbox);
//         }
//     }

//     inboxes
// }

pub async fn note(
    conn: Db,
    faktory: FaktoryConnection,
    events: EventChannels,
    note: ApNote,
    profile: Profile,
) -> Result<Status, Status> {
    // ApNote -> NewNote -> ApNote -> ApActivity
    // UUID is set in NewNote
    let n = NewNote::from((note.clone(), profile.id));

    if let Some(created_note) = create_note(&conn, n.clone()).await {
        let ap_note = ApNote::from(created_note.clone());

        log::debug!("created_note\n{created_note:#?}");

        let mut events = events;
        events.send(serde_json::to_string(&ap_note).unwrap());

        match assign_to_faktory(
            faktory,
            String::from("process_outbound_note"),
            vec![created_note.uuid],
        ) {
            Ok(_) => Ok(Status::Accepted),
            Err(_) => Err(Status::NoContent),
        }
    } else {
        Err(Status::NoContent)
    }
}

pub async fn encrypted_note(
    conn: Db,
    faktory: FaktoryConnection,
    events: EventChannels,
    note: ApNote,
    profile: Profile,
) -> Result<Status, Status> {
    // ApNote -> NewNote -> ApNote -> ApActivity
    // UUID is set in NewNote
    let n = NewNote::from((note.clone(), profile.id));

    if let Some(created_note) = create_note(&conn, n.clone()).await {
        log::debug!("created_note\n{created_note:#?}");

        // let ap_note = ApNote::from(created_note.clone());
        // let mut events = events;
        // events.send(serde_json::to_string(&ap_note).unwrap());

        match assign_to_faktory(
            faktory,
            String::from("process_outbound_note"),
            vec![created_note.uuid],
        ) {
            Ok(_) => Ok(Status::Accepted),
            Err(_) => Err(Status::NoContent),
        }
    } else {
        Err(Status::NoContent)
    }
}

pub async fn session(
    conn: Db,
    faktory: FaktoryConnection,
    session: ApSession,
    profile: Profile,
) -> Result<Status, Status> {
    let encrypted_session: NewEncryptedSession = (session.clone(), profile.id).into();

    if let Some(session) = create_encrypted_session(&conn, encrypted_session.clone()).await {
        match assign_to_faktory(faktory, String::from("send_kexinit"), vec![session.uuid]) {
            Ok(_) => Ok(Status::Accepted),
            Err(_) => Err(Status::NoContent),
        }
    } else {
        Err(Status::NoContent)
    }
}
