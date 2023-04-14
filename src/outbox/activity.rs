use crate::{
    activity_pub::{sender, ApActivity, ApDelete, ApIdentifier, ApObject},
    db::{create_leader, delete_leader, get_leader_by_profile_id_and_ap_id, Db},
    fairings::{
        events::EventChannels,
        faktory::{assign_to_faktory, FaktoryConnection},
    },
    helper::{get_local_identifier, LocalIdentifier, LocalIdentifierType},
    models::{
        announces::{create_announce, NewAnnounce},
        leaders::NewLeader,
        likes::{create_like, NewLike},
        notes::get_note_by_uuid,
        profiles::Profile,
        remote_activities::create_remote_activity,
        remote_actors::get_remote_actor_by_ap_id,
    },
    MaybeReference,
};
use log::debug;
use rocket::http::Status;

pub async fn undo_follow(
    conn: Db,
    events: EventChannels,
    mut activity: ApActivity,
    profile: Profile,
    ap_id: String,
) -> bool {
    if let Some(leader) = get_leader_by_profile_id_and_ap_id(&conn, profile.id, ap_id.clone()).await
    {
        debug!("LEADER RETRIEVED: {leader:#?}");
        let locator = format!("{}/leader/{}", *crate::SERVER_URL, leader.uuid);
        activity.object = ApObject::Identifier(ApIdentifier { id: locator });

        if let Some(actor) = get_remote_actor_by_ap_id(&conn, ap_id).await {
            if sender::send_activity(activity.clone(), profile, actor.inbox)
                .await
                .is_ok()
            {
                debug!("UNDO FOLLOW REQUEST SENT");
                if delete_leader(&conn, leader.id).await.is_ok() {
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
    events: EventChannels,
    mut activity: ApActivity,
    profile: Profile,
) -> Result<Status, Status> {
    activity.actor = format!("{}/user/{}", *crate::SERVER_URL, profile.username);

    if let ApObject::Plain(ap_id) = activity.clone().object {
        if undo_follow(conn, events, activity.clone(), profile, ap_id).await {
            Ok(Status::Accepted)
        } else {
            log::warn!("UNDO ACTION MAY BE UNIMPLEMENTED");
            log::debug!("ACTIVITY\n{activity:#?}");
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

pub async fn follow(
    conn: Db,
    events: EventChannels,
    mut activity: ApActivity,
    profile: Profile,
) -> Result<Status, Status> {
    activity.actor = format!("{}/user/{}", *crate::SERVER_URL, profile.username);

    let mut leader = NewLeader::from(activity.clone());
    leader.profile_id = profile.id;

    if let Some(leader) = create_leader(&conn, leader).await {
        debug!("leader created: {}", leader.uuid);
        activity.id = Option::from(format!("{}/leader/{}", *crate::SERVER_URL, leader.uuid));

        if create_remote_activity(&conn, activity.clone().into())
            .await
            .is_some()
        {
            debug!("updated activity\n{:#?}", activity);

            if let ApObject::Plain(object) = activity.clone().object {
                if let Some(actor) = get_remote_actor_by_ap_id(&conn, object).await {
                    if sender::send_activity(activity.clone(), profile, actor.inbox)
                        .await
                        .is_ok()
                    {
                        debug!("sent follow request successfully");

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
    } else {
        Err(Status::NoContent)
    }
}

pub async fn like(
    conn: Db,
    faktory: FaktoryConnection,
    mut activity: ApActivity,
    profile: Profile,
) -> Result<Status, Status> {
    // we ignore whatever was submitted for the actor value and enforce the correct
    // value here; this will be used in the conversion to an ApLike
    activity.actor = format!("{}/user/{}", *crate::SERVER_URL, profile.username);

    // ApLike messages do not include the "to" field, but that field is necessary for
    // delivery; so we store that as "ap_to" in the Like model, and we derive the NewLike
    // from the ApActivity directly rather than through the ApLike

    // the client sends the ApLike as an ApActivity to include the "to" field data
    if let Ok(mut like) = NewLike::try_from(activity) {
        like.link(&conn).await;

        if let Some(like) = create_like(&conn, like).await {
            log::debug!("LIKE CREATED: {}", like.uuid);

            match assign_to_faktory(faktory, String::from("send_like"), vec![like.uuid]) {
                Ok(_) => Ok(Status::Accepted),
                Err(_) => Err(Status::NoContent),
            }
        } else {
            log::error!("FAILED TO CREATE LIKE");
            Err(Status::NoContent)
        }
    } else {
        log::error!("FAILED TO CONVERT ACTIVITY TO LIKE");
        Err(Status::NoContent)
    }
}

pub async fn announce(
    conn: Db,
    faktory: FaktoryConnection,
    mut activity: ApActivity,
    profile: Profile,
) -> Result<Status, Status> {
    // we ignore whatever was submitted for the actor value and enforce the correct
    // value here; this will be used in the conversion to an ApAnnounce
    activity.actor = format!("{}/user/{}", *crate::SERVER_URL, profile.username);

    if let Ok(mut announce) = NewAnnounce::try_from(activity) {
        announce.link(&conn).await;

        if let Some(announce) = create_announce(&conn, announce).await {
            log::debug!("ANNOUNCE CREATED: {}", announce.uuid);

            match assign_to_faktory(faktory, String::from("send_announce"), vec![announce.uuid]) {
                Ok(_) => Ok(Status::Accepted),
                Err(_) => Err(Status::NoContent),
            }
        } else {
            log::error!("FAILED TO CREATE ANNOUNCE");
            Err(Status::NoContent)
        }
    } else {
        log::error!("FAILED TO CONVERT ACTIVITY TO ANNOUNCE");
        Err(Status::NoContent)
    }
}

pub async fn delete(
    conn: Db,
    faktory: FaktoryConnection,
    mut delete: ApActivity,
    profile: Profile,
) -> Result<Status, Status> {
    // we ignore whatever was submitted for the actor value and enforce the correct
    // value here
    delete.actor = format!("{}/user/{}", *crate::SERVER_URL, profile.username);

    if let ApObject::Plain(id) = delete.object {
        if let Some(identifier) = get_local_identifier(id) {
            match identifier.kind {
                LocalIdentifierType::Note => {
                    let uuid = identifier.identifier;

                    if let Some(note) = get_note_by_uuid(&conn, uuid.clone()).await {
                        if note.profile_id == profile.id {
                            match assign_to_faktory(
                                faktory,
                                String::from("delete_note"),
                                vec![uuid],
                            ) {
                                Ok(_) => Ok(Status::Accepted),
                                Err(_) => {
                                    log::error!("FAILED TO ASSIGN DELETE TO FAKTORY");
                                    Err(Status::NoContent)
                                }
                            }
                        } else {
                            log::error!("DELETE OBJECT NOT OWNED BY ACTOR");
                            Err(Status::NoContent)
                        }
                    } else {
                        log::error!("FAILED TO LOCATE NOTE BY UUID");
                        Err(Status::NoContent)
                    }
                }
                _ => {
                    log::error!("DELETE ACTION UNIMPLEMENTED FOR OBJECT TYPE");
                    Err(Status::NoContent)
                }
            }
        } else {
            log::error!("FAILED TO EXTRACT UUID FROM AP_ID");
            Err(Status::NoContent)
        }
    } else {
        log::error!("DELETE OBJECT DOES NOT LOOK LIKE A REFERENCE");
        Err(Status::NoContent)
    }
}
