use crate::{
    activity_pub::{ApObject, ApCollection, ApSession, ApNote},
    db::{
        get_encrypted_sessions_by_profile_id,
        get_remote_notes_by_profile_id,
        Db,
    },
    models::profiles::Profile,
};

pub async fn encrypted_sessions(conn: Db, profile: Profile) -> ApObject {
    let sessions = get_encrypted_sessions_by_profile_id(&conn, profile.id).await;

    let collection: Vec<ApObject> = sessions.iter().map(|x| { ApObject::Session(ApSession::from(x.clone())) }).collect();
    
    ApObject::Collection(ApCollection::from(collection))
}

pub async fn remote_notes(conn: Db, profile: Profile) -> ApObject {
    let remote_notes = get_remote_notes_by_profile_id(&conn, profile.id).await;

    let collection: Vec<ApObject> = remote_notes.iter().map(|x| { ApObject::Note(ApNote::from(x.clone())) }).collect();
    
    ApObject::Collection(ApCollection::from(collection))
}

pub async fn all(conn: Db, profile: Profile) -> ApObject {
    let mut consolidated: Vec<ApObject> = vec![];
    
    let remote_notes = get_remote_notes_by_profile_id(&conn, profile.id).await;
    let remote_notes_collection: Vec<ApObject> = remote_notes.iter().map(|x| {
        let mut note = ApNote::from(x.clone());
        note.context = Option::None;
        ApObject::Note(note)
    }).collect();
    consolidated.extend(remote_notes_collection);

    let sessions = get_encrypted_sessions_by_profile_id(&conn, profile.id).await;
    let sessions_collection: Vec<ApObject> = sessions.iter().map(|x| {
        let mut session = ApSession::from(x.clone());
        session.context = Option::None;
        ApObject::Session(session)
    }).collect();
    consolidated.extend(sessions_collection);

    
    ApObject::Collection(ApCollection::from(consolidated))
}


