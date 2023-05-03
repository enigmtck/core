use crate::{
    activity_pub::{sender, ApActivity, ApAddress, ApAnnounce, ApDelete, ApFollow, ApLike, ApUndo},
    db::Db,
    fairings::{
        events::EventChannels,
        faktory::{assign_to_faktory, FaktoryConnection},
    },
    helper::{get_ap_id_from_username, get_local_identifier, is_local, LocalIdentifierType},
    models::{
        activities::{create_activity, ActivityType, NewActivity},
        follows::get_follow_by_ap_object_and_profile,
        leaders::delete_leader_by_ap_id_and_profile,
        notes::{get_note_by_uuid, Note},
        profiles::{get_profile_by_username, Profile},
        remote_actors::{get_remote_actor_by_ap_id, RemoteActor},
        remote_notes::{get_remote_note_by_ap_id, RemoteNote},
    },
    Identifier, MaybeReference,
};
use log::debug;
use rocket::http::Status;

pub async fn undo_follow(
    conn: Db,
    events: EventChannels,
    mut activity: ApUndo,
    profile: Profile,
    ap_id: String,
) -> bool {
    if let Some(follow) =
        get_follow_by_ap_object_and_profile(&conn, ap_id.clone(), profile.id).await
    {
        debug!("FOLLOW RETRIEVED: {follow:#?}");
        let locator = format!("{}/follows/{}", *crate::SERVER_URL, follow.uuid);
        activity.object = MaybeReference::Identifier(Identifier { id: locator });

        if let Some(actor) = get_remote_actor_by_ap_id(&conn, ap_id.clone()).await {
            if sender::send_activity(
                ApActivity::Undo(Box::new(activity.clone())),
                profile.clone(),
                actor.inbox,
            )
            .await
            .is_ok()
            {
                debug!("UNDO FOLLOW REQUEST SENT");
                if delete_leader_by_ap_id_and_profile(&conn, ap_id, profile.id).await {
                    debug!("LEADER RECORD DELETED");

                    let mut events = events;
                    events.send(serde_json::to_string(&activity).unwrap());

                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
}

pub async fn undo(
    conn: Db,
    faktory: FaktoryConnection,
    activity: ApUndo,
    profile: Profile,
) -> Result<Status, Status> {
    if let MaybeReference::Actual(object) = activity.object.clone() {
        match object {
            ApActivity::Follow(follow) => {
                if let MaybeReference::Reference(leader) = follow.object {
                    if let Some(follow) =
                        get_follow_by_ap_object_and_profile(&conn, leader, profile.id).await
                    {
                        match assign_to_faktory(
                            faktory,
                            String::from("process_undo_follow"),
                            vec![follow.uuid],
                        ) {
                            Ok(_) => Ok(Status::Accepted),
                            Err(_) => Err(Status::NoContent),
                        }
                    } else {
                        Err(Status::NoContent)
                    }
                } else {
                    Err(Status::NoContent)
                }
            }
            _ => {
                log::warn!("UNDO ACTION MAY BE UNIMPLEMENTED");
                log::debug!("ACTIVITY\n{activity:#?}");
                Err(Status::NoContent)
            }
        }
    } else {
        Err(Status::NoContent)
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

pub async fn follow(
    conn: Db,
    faktory: FaktoryConnection,
    follow: ApFollow,
    profile: Profile,
) -> Result<Status, Status> {
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
                if assign_to_faktory(faktory, String::from("process_follow"), vec![activity.uuid])
                    .is_ok()
                {
                    Ok(Status::Accepted)
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

pub async fn like(
    conn: Db,
    faktory: FaktoryConnection,
    like: ApLike,
    profile: Profile,
) -> Result<Status, Status> {
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
                if assign_to_faktory(faktory, String::from("send_like"), vec![activity.uuid])
                    .is_ok()
                {
                    Ok(Status::Accepted)
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
) -> Result<Status, Status> {
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
                if assign_to_faktory(faktory, String::from("send_announce"), vec![activity.uuid])
                    .is_ok()
                {
                    Ok(Status::Accepted)
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
) -> Result<Status, Status> {
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
                match assign_to_faktory(faktory, String::from("delete_note"), vec![activity.uuid]) {
                    Ok(_) => Ok(Status::Accepted),
                    Err(_) => {
                        log::error!("FAILED TO ASSIGN DELETE TO FAKTORY");
                        Err(Status::NoContent)
                    }
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
