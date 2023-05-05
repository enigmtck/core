use crate::{
    activity_pub::{ApActivity, ApAddress, ApAnnounce, ApDelete, ApFollow, ApLike, ApUndo},
    db::Db,
    fairings::faktory::{assign_to_faktory, FaktoryConnection},
    helper::{
        get_activity_ap_id_from_uuid, get_ap_id_from_username, get_local_identifier, is_local,
        LocalIdentifierType,
    },
    models::{
        activities::{create_activity, get_activity_by_uuid, ActivityType, NewActivity},
        notes::{get_note_by_uuid, Note},
        profiles::{get_profile_by_username, Profile},
        remote_actors::{get_remote_actor_by_ap_id, RemoteActor},
        remote_notes::{get_remote_note_by_ap_id, RemoteNote},
    },
    MaybeReference,
};

use rocket::http::Status;

async fn get_notey(conn: &Db, id: String) -> (Option<Note>, Option<RemoteNote>) {
    if is_local(id.clone()) {
        if let Some(identifier) = get_local_identifier(id.clone()) {
            if identifier.kind == LocalIdentifierType::Note {
                (get_note_by_uuid(conn, identifier.identifier).await, None)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    } else {
        (None, get_remote_note_by_ap_id(conn, id).await)
    }
}

async fn get_actory(conn: &Db, id: String) -> (Option<Profile>, Option<RemoteActor>) {
    if is_local(id.clone()) {
        if let Some(identifier) = get_local_identifier(id.clone()) {
            if identifier.kind == LocalIdentifierType::User {
                (
                    get_profile_by_username(conn, identifier.identifier).await,
                    None,
                )
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    } else {
        (None, get_remote_actor_by_ap_id(conn, id).await)
    }
}

fn get_uuid(id: String) -> Option<String> {
    if let Some(identifier) = get_local_identifier(id) {
        if identifier.kind == LocalIdentifierType::Activity {
            Some(identifier.identifier)
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn undo(
    conn: Db,
    faktory: FaktoryConnection,
    undo: ApUndo,
    profile: Profile,
) -> Result<String, Status> {
    let target_ap_id = match undo.object {
        MaybeReference::Actual(object) => match object {
            ApActivity::Follow(follow) => follow.id.and_then(get_uuid),
            ApActivity::Like(like) => like.id.and_then(get_uuid),
            ApActivity::Announce(announce) => announce.id.and_then(get_uuid),
            _ => None,
        },
        _ => None,
    };

    log::debug!("TARGET_AP_ID: {target_ap_id:#?}");
    if let Some(target_ap_id) = target_ap_id {
        if let Some((target_activity, _, _, _, _)) = get_activity_by_uuid(&conn, target_ap_id).await
        {
            if let Some(activity) = create_activity(
                &conn,
                NewActivity::from((
                    target_activity,
                    ActivityType::Undo,
                    ApAddress::Address(get_ap_id_from_username(profile.username.clone())),
                ))
                .link(&conn)
                .await,
            )
            .await
            {
                if assign_to_faktory(
                    faktory,
                    String::from("process_undo"),
                    vec![activity.uuid.clone()],
                )
                .is_ok()
                {
                    Ok(get_activity_ap_id_from_uuid(activity.uuid))
                } else {
                    log::error!("FAILED TO ASSIGN UNDO TO FAKTORY");
                    Err(Status::NoContent)
                }
            } else {
                log::error!("FAILED TO CREATE UNDO ACTIVITY");
                Err(Status::NoContent)
            }
        } else {
            log::error!("FAILED TO RETRIEVE TARGET ACTIVITY");
            Err(Status::NoContent)
        }
    } else {
        log::error!("FAILED TO CONVERT OBJECT TO RELEVANT ACTIVITY");
        Err(Status::NoContent)
    }
}

pub async fn follow(
    conn: Db,
    faktory: FaktoryConnection,
    follow: ApFollow,
    profile: Profile,
) -> Result<String, Status> {
    if let MaybeReference::Reference(id) = follow.object {
        let (actor, remote_actor) = get_actory(&conn, id).await;

        if actor.is_some() || remote_actor.is_some() {
            if let Some(activity) = create_activity(
                &conn,
                NewActivity::from((
                    actor.clone(),
                    remote_actor.clone(),
                    ActivityType::Follow,
                    ApAddress::Address(get_ap_id_from_username(profile.username.clone())),
                ))
                .link(&conn)
                .await,
            )
            .await
            {
                if assign_to_faktory(
                    faktory,
                    String::from("process_follow"),
                    vec![activity.uuid.clone()],
                )
                .is_ok()
                {
                    Ok(get_activity_ap_id_from_uuid(activity.uuid))
                } else {
                    log::error!("FAILED TO ASSIGN FOLLOW TO FAKTORY");
                    Err(Status::NoContent)
                }
            } else {
                log::error!("FAILED TO CREATE FOLLOW ACTIVITY");
                Err(Status::NoContent)
            }
        } else {
            log::error!("ACTOR AND REMOTE_ACTOR CANNOT BOTH BE NONE");
            Err(Status::NoContent)
        }
    } else {
        log::error!("FOLLOW OBJECT IS NOT A REFERENCE");
        Err(Status::NoContent)
    }
}

pub async fn like(
    conn: Db,
    faktory: FaktoryConnection,
    like: ApLike,
    profile: Profile,
) -> Result<String, Status> {
    if let MaybeReference::Reference(id) = like.object {
        let (note, remote_note) = get_notey(&conn, id).await;

        if note.is_some() || remote_note.is_some() {
            if let Some(activity) = create_activity(
                &conn,
                NewActivity::from((
                    note.clone(),
                    remote_note.clone(),
                    ActivityType::Like,
                    ApAddress::Address(get_ap_id_from_username(profile.username.clone())),
                ))
                .link(&conn)
                .await,
            )
            .await
            {
                if assign_to_faktory(
                    faktory,
                    String::from("send_like"),
                    vec![activity.uuid.clone()],
                )
                .is_ok()
                {
                    Ok(get_activity_ap_id_from_uuid(activity.uuid))
                } else {
                    log::error!("FAILED TO ASSIGN LIKE TO FAKTORY");
                    Err(Status::NoContent)
                }
            } else {
                log::error!("FAILED TO CREATE LIKE ACTIVITY");
                Err(Status::NoContent)
            }
        } else {
            log::error!("NOTE AND REMOTE_NOTE CANNOT BOTH BE NONE");
            Err(Status::NoContent)
        }
    } else {
        log::error!("LIKE OBJECT IS NOT A REFERENCE");
        Err(Status::NoContent)
    }
}

pub async fn announce(
    conn: Db,
    faktory: FaktoryConnection,
    announce: ApAnnounce,
    profile: Profile,
) -> Result<String, Status> {
    if let MaybeReference::Reference(id) = announce.object {
        let (note, remote_note) = get_notey(&conn, id).await;

        if note.is_some() || remote_note.is_some() {
            if let Some(activity) = create_activity(
                &conn,
                NewActivity::from((
                    note.clone(),
                    remote_note.clone(),
                    ActivityType::Announce,
                    ApAddress::Address(get_ap_id_from_username(profile.username.clone())),
                ))
                .link(&conn)
                .await,
            )
            .await
            {
                if assign_to_faktory(
                    faktory,
                    String::from("send_announce"),
                    vec![activity.uuid.clone()],
                )
                .is_ok()
                {
                    Ok(get_activity_ap_id_from_uuid(activity.uuid))
                } else {
                    log::error!("FAILED TO ASSIGN ANNOUNCE TO FAKTORY");
                    Err(Status::NoContent)
                }
            } else {
                log::error!("FAILED TO CREATE ANNOUNCE ACTIVITY");
                Err(Status::NoContent)
            }
        } else {
            log::error!("NOTE AND REMOTE_NOTE CANNOT BOTH BE NONE");
            Err(Status::NoContent)
        }
    } else {
        log::error!("ANNOUNCE OBJECT IS NOT A REFERENCE");
        Err(Status::NoContent)
    }
}

pub async fn delete(
    conn: Db,
    faktory: FaktoryConnection,
    delete: ApDelete,
    profile: Profile,
) -> Result<String, Status> {
    if let MaybeReference::Reference(id) = delete.object {
        if let (Some(note), None) = get_notey(&conn, id).await {
            if let Some(activity) = create_activity(
                &conn,
                NewActivity::from((
                    Some(note.clone()),
                    None,
                    ActivityType::Delete,
                    ApAddress::Address(get_ap_id_from_username(profile.username.clone())),
                ))
                .link(&conn)
                .await,
            )
            .await
            {
                if assign_to_faktory(
                    faktory,
                    String::from("delete_note"),
                    vec![activity.uuid.clone()],
                )
                .is_ok()
                {
                    Ok(get_activity_ap_id_from_uuid(activity.uuid))
                } else {
                    log::error!("FAILED TO ASSIGN DELETE TO FAKTORY");
                    Err(Status::NoContent)
                }
            } else {
                log::error!("FAILED CREATE DELETE ACTIVITY");
                Err(Status::NoContent)
            }
        } else {
            log::error!("FAILED TO RETRIEVE DELETE TARGET NOTE BY UUID");
            Err(Status::NoContent)
        }
    } else {
        log::error!("DELETE OBJECT IS NOT A REFERENCE");
        Err(Status::NoContent)
    }
}
