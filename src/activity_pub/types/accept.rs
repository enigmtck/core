use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApFollow, ApObject, Inbox, Outbox},
    db::Db,
    fairings::{events::EventChannels, faktory::FaktoryConnection},
    models::{
        activities::{
            create_activity, get_activity_by_apid, ActivityTarget, ApActivityTarget, NewActivity,
        },
        profiles::Profile,
    },
    to_faktory,
    //    models::remote_activities::RemoteActivity,
    MaybeReference,
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::activity::RecursiveActivity;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApAcceptType {
    #[default]
    Accept,
}

impl fmt::Display for ApAcceptType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApAccept {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApAcceptType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub object: MaybeReference<ApActivity>,
}

impl Inbox for Box<ApAccept> {
    async fn inbox(
        &self,
        conn: Db,
        faktory: FaktoryConnection,
        raw: Value,
    ) -> Result<Status, Status> {
        //inbox::activity::accept(conn, faktory, *self.clone()).await
        let follow_apid = match self.clone().object {
            MaybeReference::Reference(reference) => Some(reference),
            MaybeReference::Actual(ApActivity::Follow(actual)) => actual.id,
            _ => None,
        };

        if let Some(follow_apid) = follow_apid {
            if let Some(target) = get_activity_by_apid(&conn, follow_apid).await {
                if let Ok(activity) = NewActivity::try_from((
                    ApActivity::Accept(self.clone()),
                    Some(ActivityTarget::from(target.0)),
                ) as ApActivityTarget)
                {
                    log::debug!("ACTIVITY\n{activity:#?}");
                    if create_activity((&conn).into(), activity.clone())
                        .await
                        .is_some()
                    {
                        to_faktory(faktory, "process_accept", activity.uuid)
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
            log::error!("{raw}");
            Err(Status::NoContent)
        }
    }
}

impl Outbox for Box<ApAccept> {
    async fn outbox(
        &self,
        _conn: Db,
        _faktory: FaktoryConnection,
        _events: EventChannels,
        _profile: Profile,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl TryFrom<RecursiveActivity> for ApAccept {
    type Error = &'static str;

    fn try_from(
        ((activity, _note, _remote_note, _profile, _remote_actor), recursive): RecursiveActivity,
    ) -> Result<Self, Self::Error> {
        if let Some(recursive) = recursive {
            if let Ok(recursive_activity) = ApActivity::try_from((recursive.clone(), None)) {
                match recursive_activity {
                    ApActivity::Follow(follow) => Ok(ApAccept {
                        context: Some(ApContext::default()),
                        kind: ApAcceptType::default(),
                        actor: activity.actor.clone().into(),
                        id: activity.ap_id.map_or(
                            Some(format!(
                                "{}/activities/{}",
                                *crate::SERVER_URL,
                                activity.uuid
                            )),
                            Some,
                        ),
                        object: MaybeReference::Actual(ApActivity::Follow(follow)),
                    }),
                    _ => {
                        log::error!("FAILED TO MATCH IMPLEMENTED ACCEPT: {activity:#?}");
                        Err("FAILED TO MATCH IMPLEMENTED ACCEPT")
                    }
                }
            } else {
                log::error!("FAILED TO CONVERT ACTIVITY: {recursive:#?}");
                Err("FAILED TO CONVERT ACTIVITY")
            }
        } else {
            log::error!("RECURSIVE CANNOT BE NONE");
            Err("RECURSIVE CANNOT BE NONE")
        }
    }
}

// impl TryFrom<RemoteActivity> for ApAccept {
//     type Error = &'static str;

//     fn try_from(activity: RemoteActivity) -> Result<Self, Self::Error> {
//         if activity.kind == "Accept" {
//             Ok(ApAccept {
//                 context: activity
//                     .context
//                     .map(|ctx| serde_json::from_value(ctx).unwrap()),
//                 kind: ApAcceptType::default(),
//                 actor: ApAddress::Address(activity.actor),
//                 id: Some(activity.ap_id),
//                 object: serde_json::from_value(activity.ap_object.into()).unwrap(),
//             })
//         } else {
//             Err("ACTIVITY COULD NOT BE CONVERTED TO ACCEPT")
//         }
//     }
// }

impl TryFrom<ApFollow> for ApAccept {
    type Error = &'static str;

    fn try_from(follow: ApFollow) -> Result<Self, Self::Error> {
        let actor = {
            match follow.object.clone() {
                MaybeReference::Actual(ApObject::Actor(actual)) => actual.id,
                MaybeReference::Reference(reference) => Some(ApAddress::Address(reference)),
                _ => None,
            }
        };

        if let Some(actor) = actor {
            Ok(ApAccept {
                context: Some(ApContext::default()),
                kind: ApAcceptType::default(),
                actor,
                id: follow.id.clone().map(|id| format!("{id}#accept")),
                object: MaybeReference::Actual(ApActivity::Follow(follow)),
            })
        } else {
            Err("COULD NOT IDENTIFY ACTOR")
        }
    }
}
