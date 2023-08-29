use crate::{
    activity_pub::{
        ApAccept, ApActivity, ApAdd, ApAddress, ApAnnounce, ApBlock, ApCreate, ApDelete, ApFollow,
        ApInvite, ApJoin, ApLike, ApObject, ApRemove, ApUndo, ApUpdate,
    },
    db::{create_remote_encrypted_session, Db},
    fairings::faktory::{assign_to_faktory, FaktoryConnection},
    models::{
        activities::{
            create_activity, get_activity_by_apid, ActivityTarget, ApActivityTarget, NewActivity,
        },
        notes::get_note_by_apid,
        profiles::get_profile_by_ap_id,
        remote_actors::{create_or_update_remote_actor, delete_remote_actor_by_ap_id},
        remote_notes::{
            create_or_update_remote_note, delete_remote_note_by_ap_id, get_remote_note_by_ap_id,
            NewRemoteNote,
        },
        timeline::delete_timeline_item_by_ap_id,
    },
    to_faktory, MaybeReference,
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
                if let Ok(activity) = NewActivity::try_from((
                    ApActivity::Create(activity),
                    Some(ActivityTarget::from(created_note.clone())),
                ) as ApActivityTarget)
                {
                    log::debug!("ACTIVITY\n{activity:#?}");
                    if create_activity(&conn, activity).await.is_some() {
                        to_faktory(faktory, "process_remote_note", created_note.ap_id)
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
    // this .link_timeline_item won't work if we don't already have the message; we'll need to
    // address that at Faktory
    // let n = NewRemoteAnnounce::from(activity.clone())
    //     .link_timeline_item(&conn)
    //     .await;

    //if let Some(created_announce) = create_remote_announce(&conn, n).await {
    if let Ok(activity) = NewActivity::try_from((ApActivity::Announce(activity), None)) {
        log::debug!("ACTIVITY\n{activity:#?}");
        if create_activity(&conn, activity.clone()).await.is_some() {
            to_faktory(faktory, "process_remote_announce", activity.uuid.clone())
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
    // } else {
    //     log::debug!("create_remote_announce failed");
    //     Err(Status::NoContent)
    // }
}

pub async fn follow(
    conn: Db,
    faktory: FaktoryConnection,
    activity: ApFollow,
) -> Result<Status, Status> {
    if let (Some(_), Some(profile_ap_id)) =
        (activity.id.clone(), activity.object.clone().reference())
    {
        if let Some(profile) = get_profile_by_ap_id(&conn, profile_ap_id.clone()).await {
            if let Ok(activity) = NewActivity::try_from((
                ApActivity::Follow(activity),
                Some(ActivityTarget::from(profile)),
            ) as ApActivityTarget)
            {
                log::debug!("ACTIVITY\n{activity:#?}");
                if let Some(activity) = create_activity(&conn, activity).await {
                    to_faktory(faktory, "acknowledge_followers", activity.uuid)
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

fn undo_target_apid(activity: &ApActivity) -> Option<String> {
    match activity {
        ApActivity::Like(like) => like.id.clone(),
        ApActivity::Follow(follow) => follow.id.clone(),
        ApActivity::Announce(announce) => announce.id.clone(),
        _ => None,
    }
}

async fn process_undo_activity(
    conn: &Db,
    faktory: FaktoryConnection,
    ap_target: &ApActivity,
    undo: &ApUndo,
) -> Result<Status, Status> {
    if let Some(ref apid) = undo_target_apid(ap_target) {
        log::debug!("APID: {apid}");
        // retrieve the activity to undo from the database (models/activities)
        if let Some(target) = get_activity_by_apid(conn, apid.clone()).await {
            log::debug!("TARGET: {target:#?}");
            // set up the parameters necessary to create an Activity in the database with linked
            // target activity; NewActivity::try_from creates the link given the appropriate database
            // in the parameterized enum
            let activity_and_target = (
                ApActivity::Undo(Box::new(undo.clone())),
                Some(ActivityTarget::from(target.0)),
            ) as ApActivityTarget;

            if let Ok(activity) = NewActivity::try_from(activity_and_target) {
                log::debug!("ACTIVITY\n{activity:#?}");
                if create_activity(conn, activity.clone()).await.is_some() {
                    match ap_target {
                        ApActivity::Like(_) => {
                            to_faktory(faktory, "process_remote_undo_like", apid.clone())
                        }
                        ApActivity::Follow(_) => {
                            to_faktory(faktory, "process_remote_undo_follow", apid.clone())
                        }
                        ApActivity::Announce(_) => {
                            to_faktory(faktory, "process_remote_undo_announce", apid.clone())
                        }
                        _ => Err(Status::NoContent),
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
    faktory: FaktoryConnection,
    activity: ApUndo,
) -> Result<Status, Status> {
    match &activity.object {
        MaybeReference::Actual(actual) => {
            process_undo_activity(&conn, faktory, actual, &activity).await
        }
        MaybeReference::Reference(_) => {
            log::warn!(
                "INSUFFICIENT CONTEXT FOR UNDO TARGET (REFERENCE FOUND WHEN ACTUAL IS REQUIRED)"
            );
            Err(Status::NoContent)
        }
        _ => {
            log::warn!("INSUFFICIENT CONTEXT FOR UNDO TARGET (NONE FOUND WHEN ACTUAL IS REQUIRED)");
            Err(Status::NoContent)
        }
    }
}

pub async fn accept(
    conn: Db,
    faktory: FaktoryConnection,
    activity: ApAccept,
) -> Result<Status, Status> {
    let follow_apid = match activity.clone().object {
        MaybeReference::Reference(reference) => Some(reference),
        MaybeReference::Actual(ApActivity::Follow(actual)) => actual.id,
        _ => None,
    };

    if let Some(follow_apid) = follow_apid {
        if let Some(target) = get_activity_by_apid(&conn, follow_apid).await {
            if let Ok(activity) = NewActivity::try_from((
                ApActivity::Accept(Box::new(activity)),
                Some(ActivityTarget::from(target.0)),
            ) as ApActivityTarget)
            {
                log::debug!("ACTIVITY\n{activity:#?}");
                if create_activity(&conn, activity.clone()).await.is_some() {
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
                log::error!("FAILED TO CONVERT ACTIVITY RECORD");
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

pub async fn like(conn: Db, _faktory: FaktoryConnection, like: ApLike) -> Result<Status, Status> {
    let note_apid = match like.object.clone() {
        MaybeReference::Reference(reference) => Some(reference),
        MaybeReference::Actual(ApObject::Note(actual)) => actual.id,
        _ => None,
    };

    if let Some(note_apid) = note_apid {
        log::debug!("NOTE AP_ID\n{note_apid:#?}");
        if let Some(target) = get_note_by_apid(&conn, note_apid).await {
            log::debug!("TARGET\n{target:#?}");
            if let Ok(activity) = NewActivity::try_from((
                ApActivity::Like(Box::new(like.clone())),
                Some(ActivityTarget::from(target)),
            ) as ApActivityTarget)
            {
                log::debug!("ACTIVITY\n{activity:#?}");
                if create_activity(&conn, activity.clone()).await.is_some() {
                    // if create_remote_like(&conn, like.clone().into())
                    //     .await
                    //     .is_some()
                    // {
                    Ok(Status::Accepted)
                    // } else {
                    //     log::warn!("FAILED TO CREATE LIKE (DUPLICATE?)\n{:#?}", like.object);
                    //     Err(Status::NoContent)
                    // }
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
