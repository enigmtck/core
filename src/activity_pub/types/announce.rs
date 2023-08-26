use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox, Temporal},
    db::Db,
    fairings::{events::EventChannels, faktory::FaktoryConnection},
    inbox,
    models::{
        activities::{ActivityType, ExtendedActivity},
        profiles::Profile,
        //announces::Announce,
        remote_announces::RemoteAnnounce,
    },
    outbox, MaybeMultiple, MaybeReference,
};
use chrono::{DateTime, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApAnnounceType {
    #[default]
    Announce,
}

impl fmt::Display for ApAnnounceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApAnnounce {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApAnnounceType,
    pub actor: ApAddress,
    pub id: Option<String>,
    pub to: MaybeMultiple<ApAddress>,
    pub cc: Option<MaybeMultiple<ApAddress>>,
    pub published: String,
    pub object: MaybeReference<ApObject>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_created_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_updated_at: Option<DateTime<Utc>>,
}

impl Inbox for ApAnnounce {
    async fn inbox(&self, conn: Db, faktory: FaktoryConnection) -> Result<Status, Status> {
        inbox::activity::announce(conn, faktory, self.clone()).await
    }
}

impl Outbox for ApAnnounce {
    async fn outbox(
        &self,
        conn: Db,
        faktory: FaktoryConnection,
        _events: EventChannels,
        profile: Profile,
    ) -> Result<String, Status> {
        outbox::activity::announce(conn, faktory, self.clone(), profile).await
    }
}

impl Temporal for ApAnnounce {
    fn published(&self) -> String {
        self.published.clone()
    }

    fn created_at(&self) -> Option<DateTime<Utc>> {
        self.ephemeral_created_at
    }

    fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.ephemeral_updated_at
    }
}

// impl From<Announce> for ApAnnounce {
//     fn from(announce: Announce) -> Self {
//         ApAnnounce {
//             context: Some(ApContext::default()),
//             kind: ApAnnounceType::default(),
//             actor: announce.actor.into(),
//             id: Some(format!(
//                 "{}/announces/{}",
//                 *crate::SERVER_URL,
//                 announce.uuid
//             )),
//             published: None,
//             object: MaybeReference::Reference(announce.object_ap_id),
//             to: serde_json::from_value(announce.ap_to).unwrap(),
//             cc: announce.cc.map(|cc| serde_json::from_value(cc).unwrap()),
//         }
//     }
// }

impl TryFrom<ExtendedActivity> for ApAnnounce {
    type Error = &'static str;

    fn try_from(
        (activity, note, remote_note, _profile, _remote_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if activity.kind == ActivityType::Announce {
            match (note, remote_note, activity.ap_to) {
                (Some(note), None, Some(ap_to)) => Ok(ApAnnounce {
                    context: Some(ApContext::default()),
                    kind: ApAnnounceType::default(),
                    actor: activity.actor.into(),
                    id: Some(format!(
                        "{}/activities/{}",
                        *crate::SERVER_URL,
                        activity.uuid
                    )),
                    to: serde_json::from_value(ap_to).unwrap(),
                    cc: activity.cc.map(|cc| serde_json::from_value(cc).unwrap()),
                    published: activity.created_at.to_rfc3339(),
                    object: MaybeReference::Reference(ApNote::from(note).id.unwrap()),
                    ephemeral_created_at: Some(activity.created_at),
                    ephemeral_updated_at: Some(activity.updated_at),
                }),
                (None, Some(remote_note), Some(ap_to)) => Ok(ApAnnounce {
                    context: Some(ApContext::default()),
                    kind: ApAnnounceType::default(),
                    actor: activity.actor.into(),
                    id: Some(format!(
                        "{}/activities/{}",
                        *crate::SERVER_URL,
                        activity.uuid
                    )),
                    to: serde_json::from_value(ap_to).unwrap(),
                    cc: activity.cc.map(|cc| serde_json::from_value(cc).unwrap()),
                    published: activity.created_at.to_rfc3339(),
                    object: MaybeReference::Reference(remote_note.ap_id),
                    ephemeral_created_at: Some(activity.created_at),
                    ephemeral_updated_at: Some(activity.updated_at),
                }),
                _ => {
                    log::error!("INVALID ACTIVITY TYPE");
                    Err("INVALID ACTIVITY TYPE")
                }
            }
        } else {
            log::error!("NOT AN ANNOUNCE ACTIVITY");
            Err("NOT AN ANNOUNCE ACTIVITY")
        }
    }
}

impl TryFrom<RemoteAnnounce> for ApAnnounce {
    type Error = &'static str;

    fn try_from(announce: RemoteAnnounce) -> Result<Self, Self::Error> {
        if let Some(ap_to) = announce.ap_to.clone() {
            Ok(ApAnnounce {
                context: Some(ApContext::default()),
                kind: ApAnnounceType::default(),
                id: Some(announce.ap_id),
                actor: ApAddress::Address(announce.actor.clone()),
                published: announce.published,
                to: serde_json::from_value::<MaybeMultiple<ApAddress>>(ap_to).unwrap(),
                cc: announce
                    .cc
                    .map(|cc| serde_json::from_value::<MaybeMultiple<ApAddress>>(cc).unwrap()),
                object: serde_json::from_value(announce.ap_object).unwrap(),
                ephemeral_created_at: Some(announce.created_at),
                ephemeral_updated_at: Some(announce.updated_at),
            })
        } else {
            Err("MISSING REQUIRED 'TO' VALUE ON REMOTE ANNOUNCE")
        }
    }
}
