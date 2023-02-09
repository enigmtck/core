use crate::{
    activity_pub::{sender, ApActivity, ApIdentifier, ApObject},
    db::{
        create_leader, create_remote_activity, delete_leader, get_leader_by_profile_id_and_ap_id,
        Db,
    },
    fairings::events::EventChannels,
    models::{leaders::NewLeader, profiles::Profile, remote_actors::get_remote_actor_by_ap_id},
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

        if create_remote_activity(&conn, (activity.clone(), profile.id).into())
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
