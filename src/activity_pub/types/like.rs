use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox},
    db::Db,
    fairings::{events::EventChannels, faktory::FaktoryConnection},
    models::{
        activities::{
            create_activity, ActivityTarget, ActivityType, ApActivityTarget, ExtendedActivity,
            NewActivity,
        },
        notes::get_note_by_apid,
        profiles::Profile,
    },
    outbox, MaybeMultiple, MaybeReference,
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApLikeType {
    #[default]
    Like,
}

impl fmt::Display for ApLikeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApLike {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApLikeType,
    pub actor: ApAddress,
    #[serde(skip_serializing)]
    pub to: Option<MaybeMultiple<ApAddress>>,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
}

impl Inbox for Box<ApLike> {
    async fn inbox(
        &self,
        conn: Db,
        _faktory: FaktoryConnection,
        raw: Value,
    ) -> Result<Status, Status> {
        let note_apid = match self.object.clone() {
            MaybeReference::Reference(reference) => Some(reference),
            MaybeReference::Actual(ApObject::Note(actual)) => actual.id,
            _ => None,
        };

        if let Some(note_apid) = note_apid {
            log::debug!("NOTE AP_ID\n{note_apid:#?}");
            if let Some(target) = get_note_by_apid(&conn, note_apid).await {
                log::debug!("TARGET\n{target:#?}");
                if let Ok(activity) = NewActivity::try_from((
                    ApActivity::Like(self.clone()),
                    Some(ActivityTarget::from(target)),
                ) as ApActivityTarget)
                {
                    log::debug!("ACTIVITY\n{activity:#?}");
                    if create_activity((&conn).into(), activity.clone())
                        .await
                        .is_ok()
                    {
                        Ok(Status::Accepted)
                    } else {
                        log::error!("FAILED TO INSERT LIKE ACTIVITY\n{raw}");
                        Err(Status::NoContent)
                    }
                } else {
                    log::error!("FAILED TO BUILD LIKE ACTIVITY\n{raw}");
                    Err(Status::NoContent)
                }
            } else {
                log::warn!("LIKED NOTE DOES NOT EXIST LOCALLY\n{raw}");
                Err(Status::NoContent)
            }
        } else {
            log::warn!("FAILED TO DETERMINE NOTE ID\n{raw}");
            Err(Status::NoContent)
        }
    }
}

impl Outbox for Box<ApLike> {
    async fn outbox(
        &self,
        conn: Db,
        faktory: FaktoryConnection,
        _events: EventChannels,
        profile: Profile,
    ) -> Result<String, Status> {
        outbox::activity::like(&conn, faktory, *self.clone(), profile).await
    }
}

impl TryFrom<ExtendedActivity> for ApLike {
    type Error = &'static str;

    fn try_from(
        (activity, note, remote_note, profile, _remote_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if activity.kind == ActivityType::Like {
            match (note, remote_note, profile) {
                (Some(note), None, None) => Ok(ApLike {
                    context: Some(ApContext::default()),
                    kind: ApLikeType::default(),
                    actor: activity.actor.into(),
                    id: Some(format!(
                        "{}/activities/{}",
                        *crate::SERVER_URL,
                        activity.uuid
                    )),
                    to: Some(MaybeMultiple::Single(ApAddress::Address(
                        note.attributed_to.clone(),
                    ))),
                    object: MaybeReference::Reference(ApNote::from(note).id.unwrap()),
                }),
                (None, Some(remote_note), None) => Ok(ApLike {
                    context: Some(ApContext::default()),
                    kind: ApLikeType::default(),
                    actor: activity.actor.into(),
                    id: Some(format!(
                        "{}/activities/{}",
                        *crate::SERVER_URL,
                        activity.uuid
                    )),
                    to: Some(MaybeMultiple::Single(ApAddress::Address(
                        remote_note.attributed_to,
                    ))),
                    object: MaybeReference::Reference(remote_note.ap_id),
                }),
                _ => {
                    log::error!("INVALID ACTIVITY TYPE");
                    Err("INVALID ACTIVITY TYPE")
                }
            }
        } else {
            log::error!("NOT A LIKE ACTIVITY");
            Err("NOT A LIKE ACTIVITY")
        }
    }
}

// impl From<Like> for ApLike {
//     fn from(like: Like) -> Self {
//         ApLike {
//             context: Some(ApContext::default()),
//             kind: ApLikeType::Like,
//             actor: ApAddress::Address(like.actor),
//             to: Some(MaybeMultiple::Single(ApAddress::Address(like.ap_to))),
//             id: Some(format!("{}/likes/{}", *crate::SERVER_URL, like.uuid)),
//             object: MaybeReference::Reference(like.object_ap_id),
//         }
//     }
// }
