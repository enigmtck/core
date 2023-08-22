use crate::{
    activity_pub::{
        ApAccept, ApActivity, ApAdd, ApAddress, ApAnnounce, ApBlock, ApCreate, ApDelete, ApFollow,
        ApInvite, ApJoin, ApLike, ApObject, ApRemove, ApUndo, ApUpdate,
    },
    db::{create_remote_encrypted_session, Db},
    fairings::{
        events::EventChannels,
        faktory::{assign_to_faktory, FaktoryConnection},
    },
    models::{
        activities::{create_activity, get_activity_by_apid, get_activity_by_uuid, NewActivity},
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

pub async fn delete(conn: Db, activity: ApDelete) -> Result<Status, Status> {
    async fn delete_actor(conn: Db, ap_id: String) -> Result<Status, Status> {
        if delete_remote_actor_by_ap_id(&conn, ap_id).await {
            debug!("REMOTE ACTOR RECORD DELETED");
            Ok(Status::Accepted)
        } else {
            Err(Status::NoContent)
        }
    }

    async fn delete_note(conn: &Db, ap_id: String) -> Result<Status, Status> {
        if delete_remote_note_by_ap_id(conn, ap_id).await {
            debug!("REMOTE NOTE RECORD DELETED");
            Ok(Status::Accepted)
        } else {
            Err(Status::NoContent)
        }
    }

    async fn delete_timeline(conn: &Db, ap_id: String) -> Result<Status, Status> {
        if delete_timeline_item_by_ap_id(conn, ap_id).await {
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
                    if remote_note.attributed_to == activity.actor.to_string() {
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
                if obj.id == activity.actor.to_string() {
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
            if ap_id == activity.actor.to_string() {
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
    activity: ApCreate,
) -> Result<Status, Status> {
    match activity.clone().object {
        MaybeReference::Actual(ApObject::Note(x)) => {
            let n = NewRemoteNote::from(x.clone());

            // creating Activity after RemoteNote is weird, but currently necessary
            // see comment in models/activities.rs on TryFrom<ApActivity>
            if let Some(created_note) = create_or_update_remote_note(&conn, n).await {
                if let Some(activity) = NewActivity::try_from(ApActivity::Create(activity))
                    .ok()
                    .map(|mut x| x.link_target(created_note.clone().into()).clone())
                {
                    log::debug!("ACTIVITY\n{activity:#?}");
                    if create_activity(&conn, activity).await.is_some() {
                        match assign_to_faktory(
                            faktory,
                            String::from("process_remote_note"),
                            vec![created_note.ap_id],
                        ) {
                            Ok(_) => Ok(Status::Accepted),
                            Err(_) => Err(Status::NoContent),
                        }
                    } else {
                        log::error!("FAILED TO INSERT ACTIVITY");
                        Err(Status::NoContent)
                    }
                } else {
                    Err(Status::NoContent)
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
    activity: ApAnnounce,
) -> Result<Status, Status> {
    // this .link won't work if we don't already have the message; we'll need to
    // address that at Faktory
    let n = NewRemoteAnnounce::from(activity.clone()).link(&conn).await;

    if let Some(created_announce) = create_remote_announce(&conn, n).await {
        if let Ok(activity) = NewActivity::try_from(ApActivity::Announce(activity)) {
            log::debug!("ACTIVITY\n{activity:#?}");
            if create_activity(&conn, activity).await.is_some() {
                match assign_to_faktory(
                    faktory,
                    String::from("process_announce"),
                    vec![created_announce.ap_id],
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
    } else {
        log::debug!("create_remote_announce failed");
        Err(Status::NoContent)
    }
}

pub async fn follow(
    conn: Db,
    faktory: FaktoryConnection,
    activity: ApFollow,
) -> Result<Status, Status> {
    if let (Some(ap_id), Some(profile_ap_id)) =
        (activity.id.clone(), activity.object.clone().reference())
    {
        if let Some(profile) = get_profile_by_ap_id(&conn, profile_ap_id.clone()).await {
            if let Some(activity) = NewActivity::try_from(ApActivity::Follow(activity))
                .ok()
                .map(|mut x| x.link_target(profile.clone().into()).clone())
            {
                log::debug!("ACTIVITY\n{activity:#?}");
                if let Some(activity) = create_activity(&conn, activity).await {
                    match assign_to_faktory(
                        faktory,
                        String::from("acknowledge_followers"),
                        vec![activity.uuid],
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

pub async fn undo(
    conn: Db,
    events: EventChannels,
    faktory: FaktoryConnection,
    activity: ApUndo,
) -> Result<Status, Status> {
    match activity.clone().object {
        MaybeReference::Actual(actual) => match actual {
            ApActivity::Like(like) => {
                if delete_remote_like_by_actor_and_object_id(
                    &conn,
                    like.actor.to_string(),
                    like.object.to_string(),
                )
                .await
                {
                    let mut events = events;
                    events.send(serde_json::to_string(&activity).unwrap());

                    Ok(Status::Accepted)
                } else {
                    Err(Status::NoContent)
                }
            }
            ApActivity::Follow(follow) => {
                if let Some(follow_apid) = follow.id {
                    if let Some(target) = get_activity_by_apid(&conn, follow_apid.clone()).await {
                        if let Some(activity) =
                            NewActivity::try_from(ApActivity::Undo(Box::new(activity)))
                                .ok()
                                .map(|mut x| x.link_target(target.0.into()).clone())
                        {
                            log::debug!("ACTIVITY\n{activity:#?}");
                            if create_activity(&conn, activity).await.is_some() {
                                match assign_to_faktory(
                                    faktory,
                                    String::from("process_remote_undo_follow"),
                                    vec![follow_apid],
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
            _ => {
                log::debug!("UNIMPLEMENTED UNDO TYPE\n{:#?}", activity.clone().object);
                Err(Status::NoContent)
            }
        },
        _ => Err(Status::NoContent),
    }
}

pub async fn accept(
    conn: Db,
    faktory: FaktoryConnection,
    activity: ApAccept,
) -> Result<Status, Status> {
    if let Some(follow_apid) = activity.object.reference() {
        if let Some(target) = get_activity_by_apid(&conn, follow_apid).await {
            if let Some(activity) = NewActivity::try_from(ApActivity::Accept(Box::new(activity)))
                .ok()
                .map(|mut x| x.link_target(target.0.into()).clone())
            {
                match assign_to_faktory(
                    faktory,
                    String::from("process_accept"),
                    vec![activity.uuid],
                ) {
                    Ok(_) => Ok(Status::Accepted),
                    Err(e) => {
                        log::error!("FAILED TO ASSIGN TO FAKTORY\n{e:#?}");
                        Err(Status::NoContent)
                    }
                }
            } else {
                log::error!("FAILED TO CREATE ACTIVITY RECORD");
                Err(Status::NoContent)
            }
        } else {
            log::error!("FAILED TO LOCATE FOLLOW ACTIVITY");
            Err(Status::NoContent)
        }
    } else {
        log::error!("FAILED TO DECODE OBJECT REFERENCE");
        Err(Status::NoContent)
    }
}

pub async fn invite(
    conn: Db,
    faktory: FaktoryConnection,
    activity: ApInvite,
) -> Result<Status, Status> {
    log::debug!("PROCESSING INVITE\n{activity:#?}");

    if let Some(ApAddress::Address(to)) = activity.to.single() {
        if let Some(profile) = get_profile_by_ap_id(&conn, to).await {
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
    activity: ApJoin,
) -> Result<Status, Status> {
    log::debug!("PROCESSING JOIN ACTIVITY\n{activity:#?}");

    if let Some(ApAddress::Address(to)) = activity.to.clone().single() {
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
    activity: ApUpdate,
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
    _faktory: FaktoryConnection,
    activity: ApLike,
) -> Result<Status, Status> {
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

pub async fn block(
    conn: Db,
    _faktory: FaktoryConnection,
    activity: ApBlock,
) -> Result<Status, Status> {
    log::warn!("BLOCK ACTIVITY NOT YET IMPLEMENTED");
    Err(Status::NoContent)
}

pub async fn add(conn: Db, _faktory: FaktoryConnection, activity: ApAdd) -> Result<Status, Status> {
    log::warn!("ADD ACTIVITY NOT YET IMPLEMENTED");
    Err(Status::NoContent)
}

pub async fn remove(
    conn: Db,
    _faktory: FaktoryConnection,
    activity: ApRemove,
) -> Result<Status, Status> {
    log::warn!("REMOVE ACTIVITY NOT YET IMPLEMENTED");
    Err(Status::NoContent)
}
