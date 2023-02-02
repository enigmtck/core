use crate::{
    activity_pub::{sender, ApActivity, ApIdentifier, ApObject},
    db::{
        create_leader, create_remote_activity, delete_leader, get_leader_by_profile_id_and_ap_id,
        get_remote_actor_by_ap_id, Db,
    },
    fairings::events::EventChannels,
    models::{leaders::NewLeader, profiles::Profile},
};
use log::debug;
use rocket::http::Status;

pub async fn undo(
    conn: Db,
    events: EventChannels,
    mut activity: ApActivity,
    profile: Profile,
) -> Result<Status, Status> {
    activity.actor = format!("{}/user/{}", *crate::SERVER_URL, profile.username);

    if let ApObject::Plain(ap_id) = activity.object {
        if let Some(leader) =
            get_leader_by_profile_id_and_ap_id(&conn, profile.id, ap_id.clone()).await
        {
            // taking the leader ap_id and converting it to the leader uuid locator seems
            // like cheating here. but I'm doing it anyway.
            debug!("leader retrieved: {}", leader.uuid);
            let locator = format!("{}/leader/{}", *crate::SERVER_URL, leader.uuid);

            activity.object = ApObject::Identifier(ApIdentifier { id: locator });
            debug!("updated activity\n{:#?}", activity);

            if let Some(actor) = get_remote_actor_by_ap_id(&conn, ap_id).await {
                if sender::send_activity(activity.clone(), profile, actor.inbox)
                    .await
                    .is_ok()
                {
                    debug!("sent undo follow request successfully");
                    if delete_leader(&conn, leader.id).await.is_ok() {
                        debug!("leader record deleted successfully");

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
