use crate::{
    activity_pub::{ApActivity, ApActivityType, ApObject, ApCollection, ApSession},
    db::{
        create_encrypted_session, create_follower, create_remote_encrypted_session,
        create_remote_note, delete_follower_by_ap_id, delete_remote_actor_by_ap_id,
        get_encrypted_sessions_by_profile_id, get_remote_activity_by_ap_id, update_leader_by_uuid,
        Db,
    },
    models::{
        encrypted_sessions::{EncryptedSession, NewEncryptedSession},
        followers::NewFollower,
        profiles::Profile,
        remote_encrypted_sessions::NewRemoteEncryptedSession,
        remote_notes::NewRemoteNote,
    },
    FaktoryConnection,
};
use faktory::Job;
use log::debug;
use rocket::http::Status;

pub async fn encrypted_sessions(conn: Db, profile: Profile) -> ApObject {
    let sessions = get_encrypted_sessions_by_profile_id(&conn, profile.id).await;

    let collection: Vec<ApObject> = sessions.iter().map(|x| { ApObject::Session(ApSession::from(x.clone())) }).collect();
    
    ApObject::Collection(ApCollection::from(collection))
}
