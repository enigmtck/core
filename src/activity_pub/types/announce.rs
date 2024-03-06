use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox, Temporal},
    db::Db,
    fairings::{events::EventChannels, faktory::FaktoryConnection},
    helper::{get_activity_ap_id_from_uuid, get_ap_id_from_username},
    models::{
        activities::{create_activity, ActivityType, ExtendedActivity, NewActivity},
        notes::{get_notey, NoteLike},
        profiles::Profile, //announces::Announce,
                           //remote_announces::RemoteAnnounce,
    },
    to_faktory, MaybeMultiple, MaybeReference,
};
use chrono::{DateTime, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    async fn inbox(
        &self,
        conn: Db,
        faktory: FaktoryConnection,
        raw: Value,
    ) -> Result<Status, Status> {
        if let Ok(activity) = NewActivity::try_from((ApActivity::Announce(self.clone()), None)) {
            log::debug!("ACTIVITY\n{activity:#?}");
            if create_activity((&conn).into(), activity.clone())
                .await
                .is_ok()
            {
                to_faktory(faktory, "process_remote_announce", activity.uuid.clone())
            } else {
                log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
                Err(Status::NoContent)
            }
        } else {
            log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
            Err(Status::NoContent)
        }
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
        outbox(&conn, faktory, self.clone(), profile).await
    }
}

async fn outbox(
    conn: &Db,
    faktory: FaktoryConnection,
    announce: ApAnnounce,
    profile: Profile,
) -> Result<String, Status> {
    if let MaybeReference::Reference(id) = announce.object {
        let note_like = get_notey(conn, id).await;

        if let Some(note_like) = note_like {
            let (note, remote_note) = match note_like {
                NoteLike::Note(note) => (Some(note), None),
                NoteLike::RemoteNote(remote_note) => (None, Some(remote_note)),
            };

            if let Ok(activity) = create_activity(
                conn.into(),
                NewActivity::from((
                    note.clone(),
                    remote_note.clone(),
                    ActivityType::Announce,
                    ApAddress::Address(get_ap_id_from_username(profile.username.clone())),
                ))
                .link_profile(conn)
                .await,
            )
            .await
            {
                if to_faktory(faktory, "send_announce", activity.uuid.clone()).is_ok() {
                    Ok(get_activity_ap_id_from_uuid(activity.uuid))
                } else {
                    log::error!("FAILED TO ASSIGN ANNOUNCE TO FAKTORY");
                    Err(Status::NoContent)
                }
            } else {
                log::error!("FAILED TO CREATE ANNOUNCE ACTIVITY");
                Err(Status::NoContent)
            }
        } else {
            log::error!("NOTE AND REMOTE_NOTE CANNOT BOTH BE NONE");
            Err(Status::NoContent)
        }
    } else {
        log::error!("ANNOUNCE OBJECT IS NOT A REFERENCE");
        Err(Status::NoContent)
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
