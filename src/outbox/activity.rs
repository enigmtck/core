use crate::{
    activity_pub::{sender, ApActivity, ApAddress, ApAnnounce, ApDelete, ApFollow, ApLike, ApUndo},
    db::Db,
    fairings::{
        events::EventChannels,
        faktory::{assign_to_faktory, FaktoryConnection},
    },
    helper::{get_local_identifier, LocalIdentifierType},
    models::{
        announces::{create_announce, NewAnnounce},
        follows::{create_follow, get_follow_by_ap_object_and_profile, NewFollow},
        leaders::delete_leader_by_ap_id_and_profile,
        likes::{create_like, NewLike},
        notes::get_note_by_uuid,
        profiles::Profile,
        remote_actors::get_remote_actor_by_ap_id,
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
                if delete_leader_by_ap_id_and_profile(&conn, ap_id, profile.id)
                    .await
                    .is_ok()
                {
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

pub async fn follow(
    conn: Db,
    faktory: FaktoryConnection,
    mut activity: ApFollow,
    profile: Profile,
) -> Result<Status, Status> {
    activity.actor =
        ApAddress::Address(format!("{}/user/{}", *crate::SERVER_URL, profile.username));

    if let Ok(mut follow) = NewFollow::try_from(activity) {
        follow.link(&conn).await;

        if create_follow(&conn, follow.clone()).await.is_some() {
            log::debug!("FOLLOW CREATED: {}", follow.uuid);

            match assign_to_faktory(faktory, String::from("process_follow"), vec![follow.uuid]) {
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

pub async fn like(
    conn: Db,
    faktory: FaktoryConnection,
    mut activity: ApLike,
    profile: Profile,
) -> Result<Status, Status> {
    // we ignore whatever was submitted for the actor value and enforce the correct
    // value here; this will be used in the conversion to an ApLike
    activity.actor =
        ApAddress::Address(format!("{}/user/{}", *crate::SERVER_URL, profile.username));

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
    mut activity: ApAnnounce,
    profile: Profile,
) -> Result<Status, Status> {
    // we ignore whatever was submitted for the actor value and enforce the correct
    // value here; this will be used in the conversion to an ApAnnounce
    activity.actor =
        ApAddress::Address(format!("{}/user/{}", *crate::SERVER_URL, profile.username));

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
    mut delete: ApDelete,
    profile: Profile,
) -> Result<Status, Status> {
    // we ignore whatever was submitted for the actor value and enforce the correct
    // value here
    delete.actor = ApAddress::Address(format!("{}/user/{}", *crate::SERVER_URL, profile.username));

    if let MaybeReference::Reference(id) = delete.object {
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
