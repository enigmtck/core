use crate::{
    activity_pub::{ApActor, ApAddress, ApNote, ApSession},
    db::{create_note, Db},
    fairings::{
        events::EventChannels,
        faktory::{assign_to_faktory, FaktoryConnection},
    },
    helper::get_ap_id_from_username,
    models::{
        activities::{create_activity, ActivityType, NewActivity, NoteActivity},
        encrypted_sessions::{create_encrypted_session, NewEncryptedSession},
        notes::NewNote,
        profiles::Profile,
    },
};
use rocket::http::Status;

pub async fn note(
    conn: Db,
    faktory: FaktoryConnection,
    _events: EventChannels,
    note: ApNote,
    profile: Profile,
) -> Result<String, Status> {
    // ApNote -> NewNote -> ApNote -> ApActivity
    // UUID is set in NewNote
    let mut new_note = NewNote::from((note.clone(), profile.id));

    if let (Some(note), Some(followers)) =
        (note.to.single(), ApActor::from(profile.clone()).followers)
    {
        if note.is_public() {
            new_note.cc = Some(serde_json::to_value(vec![followers]).unwrap());
        }
    }

    if let Some(created_note) = create_note(&conn, new_note.clone()).await {
        if let Some(activity) = create_activity(
            &conn,
            NewActivity::from((
                Some(created_note.clone()),
                None,
                ActivityType::Create,
                ApAddress::Address(get_ap_id_from_username(profile.username.clone())),
            ))
            .link(&conn)
            .await,
        )
        .await
        {
            if assign_to_faktory(
                faktory,
                String::from("process_outbound_note"),
                vec![activity.uuid.clone()],
            )
            .is_ok()
            {
                Ok(activity.uuid)
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

pub async fn encrypted_note(
    conn: Db,
    faktory: FaktoryConnection,
    _events: EventChannels,
    note: ApNote,
    profile: Profile,
) -> Result<String, Status> {
    // ApNote -> NewNote -> ApNote -> ApActivity
    // UUID is set in NewNote
    let n = NewNote::from((note.clone(), profile.id));

    if let Some(created_note) = create_note(&conn, n.clone()).await {
        log::debug!("created_note\n{created_note:#?}");

        // let ap_note = ApNote::from(created_note.clone());
        // let mut events = events;
        // events.send(serde_json::to_string(&ap_note).unwrap());

        if assign_to_faktory(
            faktory,
            String::from("process_outbound_note"),
            vec![created_note.uuid.clone()],
        )
        .is_ok()
        {
            Ok(created_note.uuid)
        } else {
            Err(Status::NoContent)
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
) -> Result<String, Status> {
    let encrypted_session: NewEncryptedSession = (session.clone(), profile.id).into();

    if let Some(session) = create_encrypted_session(&conn, encrypted_session.clone()).await {
        if assign_to_faktory(
            faktory,
            String::from("send_kexinit"),
            vec![session.uuid.clone()],
        )
        .is_ok()
        {
            Ok(session.uuid)
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}
