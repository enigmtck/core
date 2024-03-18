use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox, Temporal},
    db::Db,
    fairings::events::EventChannels,
    helper::get_activity_ap_id_from_uuid,
    models::{
        activities::{create_activity, ActivityType, ExtendedActivity, NewActivity, NoteActivity},
        notes::get_notey,
        profiles::Profile,
    },
    runner, MaybeMultiple, MaybeReference,
};
use chrono::{DateTime, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApAnnounceType {
    #[default]
    #[serde(alias = "announce")]
    Announce,
}

impl fmt::Display for ApAnnounceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

// The sqlite version changes the ephemeral dates to naive, but I don't want to do that
// may need to fix this (Ap versions should be UTC while Db versions should be naive)
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
    async fn inbox(&self, conn: Db, channels: EventChannels, raw: Value) -> Result<Status, Status> {
        let activity = NewActivity::try_from((ApActivity::Announce(self.clone()), None))
            .map_err(|_| Status::new(520))?;
        log::debug!("ACTIVITY\n{activity:#?}");
        if create_activity((&conn).into(), activity.clone())
            .await
            .is_ok()
        {
            runner::run(
                runner::announce::remote_announce_task,
                Some(conn),
                Some(channels),
                vec![activity.uuid.clone()],
            )
            .await;
            Ok(Status::Accepted)
        } else {
            log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
            Err(Status::new(521))
        }
    }
}

impl Outbox for ApAnnounce {
    async fn outbox(
        &self,
        conn: Db,
        events: EventChannels,
        profile: Profile,
    ) -> Result<String, Status> {
        outbox(conn, events, self.clone(), profile).await
    }
}

async fn outbox(
    conn: Db,
    channels: EventChannels,
    announce: ApAnnounce,
    profile: Profile,
) -> Result<String, Status> {
    if let MaybeReference::Reference(id) = announce.object {
        let note_like = get_notey(&conn, id).await.ok_or(Status::new(520))?;

        let activity = create_activity(
            Some(&conn),
            NewActivity::from(NoteActivity {
                note: note_like,
                profile: profile.clone(),
                kind: ActivityType::Announce,
            })
            .link_profile(&conn)
            .await,
        )
        .await
        .map_err(|_| Status::new(521))?;

        runner::run(
            runner::announce::send_announce_task,
            Some(conn),
            Some(channels),
            vec![activity.uuid.clone()],
        )
        .await;
        Ok(get_activity_ap_id_from_uuid(activity.uuid))
    } else {
        log::error!("ANNOUNCE OBJECT IS NOT A REFERENCE");
        Err(Status::new(523))
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

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        impl TryFrom<ExtendedActivity> for ApAnnounce {
            type Error = &'static str;

            fn try_from(
                (activity, note, remote_note, _profile, _remote_actor): ExtendedActivity,
            ) -> Result<Self, Self::Error> {
                if activity.kind == crate::models::activities::ActivityType::Announce {
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
    } else if #[cfg(feature = "sqlite")] {
        impl TryFrom<ExtendedActivity> for ApAnnounce {
            type Error = &'static str;

            fn try_from(
                (activity, note, remote_note, _profile, _remote_actor): ExtendedActivity,
            ) -> Result<Self, Self::Error> {
                if activity.kind.as_str() == "announce" {
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
                            to: serde_json::from_str(&ap_to).unwrap(),
                            cc: activity.cc.map(|cc| serde_json::from_str(&cc).unwrap()),
                            published: activity.created_at.to_string(),
                            object: MaybeReference::Reference(ApNote::from(note).id.unwrap()),
                            ephemeral_created_at: { Some(DateTime::<Utc>::from_naive_utc_and_offset(
                                activity.created_at,
                                Utc,
                            )) },
                            ephemeral_updated_at: { Some(DateTime::<Utc>::from_naive_utc_and_offset(
                                activity.updated_at,
                                Utc,
                            ))},
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
                            to: serde_json::from_str(&ap_to).unwrap(),
                            cc: activity.cc.map(|cc| serde_json::from_str(&cc).unwrap()),
                            published: activity.created_at.to_string(),
                            object: MaybeReference::Reference(remote_note.ap_id),
                            ephemeral_created_at: { Some(DateTime::<Utc>::from_naive_utc_and_offset(
                                activity.created_at,
                                Utc,
                            )) },
                            ephemeral_updated_at: { Some(DateTime::<Utc>::from_naive_utc_and_offset(
                                activity.updated_at,
                                Utc,
                            ))},
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

    }
}
