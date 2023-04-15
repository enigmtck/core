use crate::{
    activity_pub::{ApActivity, ApObject},
    db::{create_remote_encrypted_session, create_remote_note, Db},
    fairings::{
        events::EventChannels,
        faktory::{assign_to_faktory, FaktoryConnection},
    },
    models::{
        profiles::get_profile_by_ap_id,
        remote_actors::{create_or_update_remote_actor, delete_remote_actor_by_ap_id},
        remote_announces::{create_remote_announce, NewRemoteAnnounce},
        remote_likes::{create_remote_like, delete_remote_like_by_actor_and_object_id},
        remote_notes::{
            create_or_update_remote_note, delete_remote_note_by_ap_id, get_remote_note_by_ap_id,
            NewRemoteNote,
        },
        timeline::delete_timeline_item_by_ap_id,
    },
    MaybeReference,
};
use log::debug;
use rocket::http::Status;

pub async fn delete(conn: Db, activity: ApActivity) -> Result<Status, Status> {
    async fn delete_actor(conn: Db, ap_id: String) -> Result<Status, Status> {
        if delete_remote_actor_by_ap_id(&conn, ap_id).await.is_ok() {
            debug!("REMOTE ACTOR RECORD DELETED");
            Ok(Status::Accepted)
        } else {
            Err(Status::NoContent)
        }
    }

    async fn delete_note(conn: &Db, ap_id: String) -> Result<Status, Status> {
        if delete_remote_note_by_ap_id(conn, ap_id).await.is_ok() {
            debug!("REMOTE NOTE RECORD DELETED");
            Ok(Status::Accepted)
        } else {
            Err(Status::NoContent)
        }
    }

    async fn delete_timeline(conn: &Db, ap_id: String) -> Result<Status, Status> {
        if delete_timeline_item_by_ap_id(conn, ap_id).await.is_ok() {
            debug!("TIMELINE RECORD DELETED");
            Ok(Status::Accepted)
        } else {
            Err(Status::NoContent)
        }
    }

    match activity.object {
        MaybeReference::Actual(actual) => match actual {
            ApObject::Tombstone(tombstone) => {
                if let Some(remote_note) =
                    get_remote_note_by_ap_id(&conn, tombstone.id.clone()).await
                {
                    if remote_note.attributed_to == activity.actor {
                        if delete_note(&conn, tombstone.id.clone()).await.is_ok() {
                            delete_timeline(&conn, tombstone.id).await
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
            ApObject::Identifier(obj) => {
                if obj.id == activity.actor {
                    delete_actor(conn, obj.id).await
                } else {
                    debug!("DOESN'T MATCH ACTOR; ASSUMING NOTE");
                    if delete_note(&conn, obj.clone().id).await.is_ok() {
                        delete_timeline(&conn, obj.id).await
                    } else {
                        Err(Status::NoContent)
                    }
                }
            }
            _ => {
                debug!("delete didn't match anything");
                Err(Status::NoContent)
            }
        },
        MaybeReference::Reference(ap_id) => {
            if ap_id == activity.actor {
                delete_actor(conn, ap_id).await
            } else {
                debug!("DOESN'T MATCH ACTOR; ASSUMING NOTE");
                if delete_note(&conn, ap_id.clone()).await.is_ok() {
                    delete_timeline(&conn, ap_id).await
                } else {
                    Err(Status::NoContent)
                }
            }
        }
        _ => Err(Status::NoContent),
    }
}

pub async fn create(
    conn: Db,
    faktory: FaktoryConnection,
    activity: ApActivity,
) -> Result<Status, Status> {
    match activity.object {
        MaybeReference::Actual(ApObject::Note(x)) => {
            let n = NewRemoteNote::from(x.clone());

            if let Some(created_note) = create_remote_note(&conn, n).await {
                match assign_to_faktory(
                    faktory,
                    String::from("process_remote_note"),
                    vec![created_note.ap_id],
                ) {
                    Ok(_) => Ok(Status::Accepted),
                    Err(_) => Err(Status::NoContent),
                }
            } else {
                Err(Status::NoContent)
            }
        }
        _ => Err(Status::NoContent),
    }
}

pub async fn announce(
    conn: Db,
    faktory: FaktoryConnection,
    activity: ApActivity,
) -> Result<Status, Status> {
    // this .link won't work if we don't already have the message; we'll need to
    // address that at Faktory
    let n = NewRemoteAnnounce::from(activity.clone()).link(&conn).await;

    if let Some(created_announce) = create_remote_announce(&conn, n).await {
        match assign_to_faktory(
            faktory,
            String::from("process_announce"),
            vec![created_announce.ap_id],
        ) {
            Ok(_) => Ok(Status::Accepted),
            Err(_) => Err(Status::NoContent),
        }
    } else {
        log::debug!("create_remote_announce failed");
        Err(Status::NoContent)
    }
}

pub async fn follow(faktory: FaktoryConnection, activity: ApActivity) -> Result<Status, Status> {
    if let Some(ap_id) = activity.id {
        match assign_to_faktory(faktory, String::from("acknowledge_followers"), vec![ap_id]) {
            Ok(_) => Ok(Status::Accepted),
            Err(e) => {
                log::error!("FAILED TO ASSIGN TO FAKTORY\n{e:#?}");
                Err(Status::NoContent)
            }
        }
    } else {
        Err(Status::NoContent)
    }
}

pub async fn undo(
    conn: Db,
    events: EventChannels,
    faktory: FaktoryConnection,
    activity: ApActivity,
) -> Result<Status, Status> {
    match activity.clone().object {
        MaybeReference::Actual(actual) => match actual {
            ApObject::Like(like) => {
                if delete_remote_like_by_actor_and_object_id(&conn, like.actor, like.object).await {
                    let mut events = events;
                    events.send(serde_json::to_string(&activity).unwrap());

                    Ok(Status::Accepted)
                } else {
                    Err(Status::NoContent)
                }
            }
            ApObject::Follow(follow) => {
                if let Some(id) = follow.id {
                    match assign_to_faktory(
                        faktory,
                        String::from("process_remote_undo_follow"),
                        vec![id],
                    ) {
                        Ok(_) => Ok(Status::Accepted),
                        Err(e) => {
                            log::error!("FAILED TO ASSIGN TO FAKTORY\n{e:#?}");
                            Err(Status::NoContent)
                        }
                    }
                } else {
                    Err(Status::NoContent)
                }
            }
            _ => {
                log::debug!("UNIMPLEMENTED UNDO TYPE\n{:#?}", activity.clone().object);
                Err(Status::NoContent)
            }
        },
        _ => Err(Status::NoContent),
    }
}

pub async fn accept(faktory: FaktoryConnection, activity: ApActivity) -> Result<Status, Status> {
    if let Some(id) = activity.id {
        match assign_to_faktory(faktory, String::from("process_accept"), vec![id]) {
            Ok(_) => Ok(Status::Accepted),
            Err(e) => {
                log::error!("FAILED TO ASSIGN TO FAKTORY\n{e:#?}");
                Err(Status::NoContent)
            }
        }
    } else {
        log::error!("COULD NOT LOCATE ID");
        Err(Status::NoContent)
    }
}

pub async fn invite(
    conn: Db,
    faktory: FaktoryConnection,
    activity: ApActivity,
) -> Result<Status, Status> {
    log::debug!("PROCESSING INVITE\n{activity:#?}");

    let invited: Option<String> = {
        if let MaybeReference::Reference(to) = activity.clone().object {
            Some(to)
        } else if let Some(to) = activity.clone().to {
            to.single().map(|single| single.to_string())
        } else {
            None
        }
    };

    if let Some(to) = invited {
        if let Some(profile) = get_profile_by_ap_id(&conn, to.clone()).await {
            if let Some(session) =
                create_remote_encrypted_session(&conn, (activity.clone(), profile.id).into()).await
            {
                match assign_to_faktory(
                    faktory,
                    String::from("provide_one_time_key"),
                    vec![session.ap_id.clone()],
                ) {
                    Ok(_) => {
                        log::debug!("ASSIGNED TO FAKTORY: {:?}", session.ap_id);
                        Ok(Status::Accepted)
                    }
                    Err(_) => Err(Status::NoContent),
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

pub async fn join(
    conn: Db,
    faktory: FaktoryConnection,
    activity: ApActivity,
) -> Result<Status, Status> {
    log::debug!("PROCESSING JOIN ACTIVITY\n{activity:#?}");

    let joined: Option<String> = {
        if let MaybeReference::Reference(to) = activity.clone().object {
            Some(to)
        } else if let Some(to) = activity.clone().to {
            to.single().map(|single| single.to_string())
        } else {
            None
        }
    };

    if let Some(to) = joined {
        if let Some(profile) = get_profile_by_ap_id(&conn, to.clone()).await {
            if create_remote_encrypted_session(&conn, (activity.clone(), profile.id).into())
                .await
                .is_some()
            {
                if let MaybeReference::Actual(ApObject::Session(session)) = activity.object {
                    if let Some(ap_id) = session.id {
                        log::debug!("ASSIGNING JOIN ACTIVITY TO FAKTORY");

                        match assign_to_faktory(faktory, String::from("process_join"), vec![ap_id])
                        {
                            Ok(_) => Ok(Status::Accepted),
                            Err(_) => Err(Status::NoContent),
                        }
                    } else {
                        log::error!("MISSING ID");
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
    } else {
        Err(Status::NoContent)
    }
}

pub async fn update(
    conn: Db,
    faktory: FaktoryConnection,
    activity: ApActivity,
) -> Result<Status, Status> {
    match activity.object {
        MaybeReference::Actual(actual) => match actual {
            ApObject::Actor(actor) => {
                log::debug!("UPDATING ACTOR: {}", actor.clone().id.unwrap_or_default());

                if actor.clone().id.unwrap_or_default() == activity.actor
                    && create_or_update_remote_actor(&conn, actor.into())
                        .await
                        .is_some()
                {
                    Ok(Status::Accepted)
                } else {
                    Err(Status::NoContent)
                }
            }
            ApObject::Note(note) => {
                if let Some(id) = note.clone().id {
                    log::debug!("UPDATING NOTE: {}", id);

                    if note.clone().attributed_to == activity.actor
                        && create_or_update_remote_note(&conn, note.into())
                            .await
                            .is_some()
                    {
                        match assign_to_faktory(
                            faktory,
                            String::from("update_timeline_record"),
                            vec![id],
                        ) {
                            Ok(_) => Ok(Status::Accepted),
                            Err(_) => Err(Status::NoContent),
                        }
                    } else {
                        Err(Status::NoContent)
                    }
                } else {
                    log::warn!("MISSING NOTE ID: {note:#?}");
                    Err(Status::NoContent)
                }
            }
            _ => {
                log::debug!("UNIMPLEMENTED UPDATE TYPE");
                Err(Status::NoContent)
            }
        },
        _ => Err(Status::NoContent),
    }
}

pub async fn like(
    conn: Db,
    faktory: FaktoryConnection,
    activity: ApActivity,
) -> Result<Status, Status> {
    match activity.object {
        MaybeReference::Reference(_) => {
            if create_remote_like(&conn, activity.clone().into())
                .await
                .is_some()
            {
                Ok(Status::Accepted)
            } else {
                log::warn!("FAILED TO CREATE LIKE (DUPLICATE?)\n{:#?}", activity.object);
                Err(Status::NoContent)
            }
        }
        _ => {
            log::warn!("UNEXPECTED OBJECT TYPE\n{:#?}", activity.object);
            Err(Status::NoContent)
        }
    }
}
