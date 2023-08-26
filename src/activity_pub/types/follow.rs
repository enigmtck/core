use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActor, ApAddress, ApContext, ApObject, Inbox, Outbox},
    db::Db,
    fairings::{events::EventChannels, faktory::FaktoryConnection},
    inbox,
    models::{
        activities::{ActivityType, ExtendedActivity},
        profiles::Profile,
    },
    outbox, MaybeReference,
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};

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
    async fn inbox(&self, conn: Db, faktory: FaktoryConnection) -> Result<Status, Status> {
        inbox::activity::follow(conn, faktory, self.clone()).await
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
        outbox::activity::follow(conn, faktory, self.clone(), profile).await
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
