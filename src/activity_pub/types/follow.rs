use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApActor, ApAddress, ApContext, ApObject, Inbox, Outbox},
    db::Db,
    fairings::{events::EventChannels, faktory::FaktoryConnection},
    helper::{get_activity_ap_id_from_uuid, get_ap_id_from_username},
    models::{
        activities::{
            create_activity, ActivityTarget, ActivityType, ApActivityTarget, ExtendedActivity,
            NewActivity,
        },
        profiles::{get_actory, get_profile_by_ap_id, ActorLike, Profile},
    },
    to_faktory, MaybeReference,
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApFollowType {
    #[default]
    Follow,
}

impl fmt::Display for ApFollowType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApFollow {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApFollowType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
}

impl Inbox for ApFollow {
    async fn inbox(
        &self,
        conn: Db,
        faktory: FaktoryConnection,
        raw: Value,
    ) -> Result<Status, Status> {
        if let (Some(_), Some(profile_ap_id)) = (self.id.clone(), self.object.clone().reference()) {
            if let Some(profile) = get_profile_by_ap_id(Some(&conn), profile_ap_id.clone()).await {
                if let Ok(activity) = NewActivity::try_from((
                    ApActivity::Follow(self.clone()),
                    Some(ActivityTarget::from(profile)),
                ) as ApActivityTarget)
                {
                    log::debug!("ACTIVITY\n{activity:#?}");
                    if let Ok(activity) = create_activity((&conn).into(), activity).await {
                        to_faktory(faktory, "acknowledge_followers", vec![activity.uuid])
                    } else {
                        log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                        Err(Status::NoContent)
                    }
                } else {
                    log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                    Err(Status::NoContent)
                }
            } else {
                log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
                Err(Status::NoContent)
            }
        } else {
            log::error!("FAILED TO HANDLE ACTIVITY\n{raw}");
            Err(Status::NoContent)
        }
    }
}

impl Outbox for ApFollow {
    async fn outbox(
        &self,
        conn: Db,
        faktory: FaktoryConnection,
        _events: EventChannels,
        profile: Profile,
    ) -> Result<String, Status> {
        outbox(&conn, faktory, self.clone(), profile).await
    }
}

async fn outbox(
    conn: &Db,
    faktory: FaktoryConnection,
    follow: ApFollow,
    profile: Profile,
) -> Result<String, Status> {
    if let MaybeReference::Reference(id) = follow.object {
        let actor_like = get_actory(conn, id).await;

        if let Some(actor_like) = actor_like {
            let (actor, remote_actor) = match actor_like {
                ActorLike::Profile(profile) => (Some(profile), None),
                ActorLike::RemoteActor(remote_actor) => (None, Some(remote_actor)),
            };

            if let Ok(activity) = create_activity(
                conn.into(),
                NewActivity::from((
                    actor.clone(),
                    remote_actor.clone(),
                    ActivityType::Follow,
                    ApAddress::Address(get_ap_id_from_username(profile.username.clone())),
                ))
                .link_profile(conn)
                .await,
            )
            .await
            {
                if to_faktory(faktory, "process_follow", vec![activity.uuid.clone()]).is_ok() {
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

impl TryFrom<ExtendedActivity> for ApFollow {
    type Error = &'static str;

    fn try_from(
        (activity, _note, _remote_note, profile, remote_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if activity.kind == ActivityType::Follow {
            match (profile, remote_actor) {
                (Some(profile), None) => Ok(ApFollow {
                    context: Some(ApContext::default()),
                    kind: ApFollowType::default(),
                    actor: activity.actor.into(),
                    id: activity.ap_id.map_or(
                        Some(format!(
                            "{}/activities/{}",
                            *crate::SERVER_URL,
                            activity.uuid
                        )),
                        Some,
                    ),
                    object: MaybeReference::Reference(
                        ApActor::from(profile).id.unwrap().to_string(),
                    ),
                }),
                (None, Some(remote_actor)) => Ok(ApFollow {
                    context: Some(ApContext::default()),
                    kind: ApFollowType::default(),
                    actor: activity.actor.into(),
                    id: activity.ap_id.map_or(
                        Some(format!(
                            "{}/activities/{}",
                            *crate::SERVER_URL,
                            activity.uuid
                        )),
                        Some,
                    ),
                    object: MaybeReference::Reference(remote_actor.ap_id),
                }),
                _ => {
                    log::error!("INVALID ACTIVITY TYPE");
                    Err("INVALID ACTIVITY TYPE")
                }
            }
        } else {
            log::error!("NOT A FOLLOW ACTIVITY");
            Err("NOT A FOLLOW ACTIVITY")
        }
    }
}
