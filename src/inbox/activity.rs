use crate::{
    activity_pub::{ApActivity, ApActivityType, ApNote, ApObject},
    db::{
        create_follower, create_remote_encrypted_session, create_remote_note,
        delete_follower_by_ap_id, update_leader_by_uuid, Db,
    },
    fairings::{
        events::EventChannels,
        faktory::{assign_to_faktory, FaktoryConnection},
    },
    models::{
        followers::NewFollower,
        profiles::get_profile_by_ap_id,
        remote_activities::get_remote_activity_by_ap_id,
        remote_actors::{create_or_update_remote_actor, delete_remote_actor_by_ap_id},
        remote_announces::{create_remote_announce, NewRemoteAnnounce},
        remote_notes::{create_or_update_remote_note, delete_remote_note_by_ap_id, NewRemoteNote},
        timeline::delete_timeline_item_by_ap_id,
    },
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
        ApObject::Plain(ap_id) => {
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
    }
}

pub async fn create(
    conn: Db,
    faktory: FaktoryConnection,
    events: EventChannels,
    activity: ApActivity,
) -> Result<Status, Status> {
    match activity.object {
        ApObject::Note(x) => {
            let n = NewRemoteNote::from(x.clone());

            if let Some(created_note) = create_remote_note(&conn, n).await {
                log::debug!("CREATED REMOTE NOTE\n{:#?}", created_note);
                log::debug!("...FROM\n{:#?}", x.clone());

                let note: ApNote = created_note.clone().into();
                let mut events = events;
                events.send(serde_json::to_string(&note).unwrap());

                match assign_to_faktory(
                    faktory,
                    String::from("process_remote_note"),
                    vec![created_note.ap_id],
                ) {
                    Ok(_) => Ok(Status::Accepted),
                    Err(_) => Err(Status::NoContent),
                }
            } else {
                //log::debug!("create_remote_note failed (probably a duplicate)");
                Err(Status::NoContent)
            }
        }
        _ => {
            log::debug!("doesn't look like a note\n{activity:#?}");
            Err(Status::NoContent)
        }
    }
}

pub async fn announce(
    conn: Db,
    faktory: FaktoryConnection,
    events: EventChannels,
    activity: ApActivity,
) -> Result<Status, Status> {
    let n = NewRemoteAnnounce::from(activity.clone());

    if let Some(created_announce) = create_remote_announce(&conn, n).await {
        let mut events = events;
        events.send(serde_json::to_string(&activity).unwrap());

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

pub async fn follow(
    conn: Db,
    faktory: FaktoryConnection,
    events: EventChannels,
    activity: ApActivity,
) -> Result<Status, Status> {
    let followed: Option<String> = {
        if let ApObject::Plain(to) = activity.clone().object {
            Some(to)
        } else if let Some(to) = activity.clone().to {
            to.single()
        } else {
            None
        }
    };

    if let Some(to) = followed {
        if let Some(profile) = get_profile_by_ap_id(&conn, to.clone()).await {
            let mut f = NewFollower::from(activity.clone());
            f.profile_id = profile.id;

            if let Some(created_follower) = create_follower(&conn, f).await {
                log::debug!("CREATED FOLLOWER\n{created_follower:#?}");

                let mut events = events;
                events.send(serde_json::to_string(&activity).unwrap());

                match assign_to_faktory(
                    faktory,
                    String::from("acknowledge_followers"),
                    vec![created_follower.uuid],
                ) {
                    Ok(_) => Ok(Status::Accepted),
                    Err(e) => {
                        log::error!("FAILED TO ASSIGN TO FAKTORY\n{e:#?}");
                        Err(Status::NoContent)
                    }
                }
            } else {
                log::error!("CREATE FOLLOWER FAILED");
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

pub async fn undo(conn: Db, events: EventChannels, activity: ApActivity) -> Result<Status, Status> {
    if let ApObject::Identifier(x) = activity.clone().object {
        if let Some(x) = get_remote_activity_by_ap_id(&conn, x.id).await {
            if x.kind == ApActivityType::Follow.to_string()
                && delete_follower_by_ap_id(&conn, x.ap_id).await
            {
                let mut events = events;
                events.send(serde_json::to_string(&activity).unwrap());

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

pub async fn accept(
    conn: Db,
    events: EventChannels,
    activity: ApActivity,
) -> Result<Status, Status> {
    //debug!("activity: {activity:#?}");

    let identifier = match activity.clone().object {
        ApObject::Identifier(x) => Option::from(x.id),
        ApObject::Plain(x) => Option::from(x),
        _ => Option::None,
    };

    if let Some(x) = identifier {
        let ap_id_re = regex::Regex::new(r#"(\w+://)(.+?/)+(.+)"#).unwrap();
        if let Some(ap_id_match) = ap_id_re.captures(&x) {
            //debug!("ap_id_match: {:#?}", ap_id_match);

            let matches = ap_id_match.len();
            let uuid = ap_id_match.get(matches - 1).unwrap().as_str();

            if let Some(id) = activity.clone().id {
                if update_leader_by_uuid(&conn, uuid.to_string(), id)
                    .await
                    .is_some()
                {
                    let mut events = events;
                    events.send(serde_json::to_string(&activity).unwrap());

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
    } else {
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
        if let ApObject::Plain(to) = activity.clone().object {
            Some(to)
        } else if let Some(to) = activity.clone().to {
            to.single()
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
        if let ApObject::Plain(to) = activity.clone().object {
            Some(to)
        } else if let Some(to) = activity.clone().to {
            to.single()
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
                if let ApObject::Session(session) = activity.object {
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
    }
}
