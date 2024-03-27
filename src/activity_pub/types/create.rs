use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox, Temporal},
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::{
            create_activity, ActivityTarget, ApActivityTarget, ExtendedActivity, NewActivity,
        },
        profiles::Profile,
        remote_notes::{create_or_update_remote_note, NewRemoteNote},
        remote_questions::create_or_update_remote_question,
    },
    runner, MaybeMultiple, MaybeReference,
};
use chrono::{DateTime, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::signature::ApSignature;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApCreateType {
    #[default]
    #[serde(alias = "create")]
    Create,
}

impl fmt::Display for ApCreateType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApCreate {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApCreateType,
    pub actor: ApAddress,
    pub to: MaybeMultiple<ApAddress>,
    pub cc: Option<MaybeMultiple<ApAddress>>,
    pub id: Option<String>,
    pub object: MaybeReference<ApObject>,
    pub published: Option<String>,
    pub signature: Option<ApSignature>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_created_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral_updated_at: Option<DateTime<Utc>>,
}

impl Inbox for ApCreate {
    async fn inbox(&self, conn: Db, channels: EventChannels, raw: Value) -> Result<Status, Status> {
        match self.clone().object {
            MaybeReference::Actual(ApObject::Note(x)) => {
                let n = NewRemoteNote::from(x.clone());

                // creating Activity after RemoteNote is weird, but currently necessary
                // see comment in models/activities.rs on TryFrom<ApActivity>
                let created_note = create_or_update_remote_note(Some(&conn), n)
                    .await
                    .ok_or(Status::new(520))?;
                let activity = NewActivity::try_from((
                    ApActivity::Create(self.clone()),
                    Some(ActivityTarget::from(created_note.clone())),
                ) as ApActivityTarget)
                .map_err(|_| Status::new(521))?;

                log::debug!("ACTIVITY\n{activity:#?}");

                if create_activity((&conn).into(), activity).await.is_ok() {
                    runner::run(
                        runner::note::remote_note_task,
                        Some(conn),
                        Some(channels),
                        vec![created_note.ap_id],
                    )
                    .await;
                    Ok(Status::Accepted)
                } else {
                    log::error!("FAILED TO INSERT ACTIVITY");
                    Err(Status::NoContent)
                }
            }
            MaybeReference::Actual(ApObject::Question(question)) => {
                let created_question = create_or_update_remote_question(&conn, question.into())
                    .await
                    .map_err(|e| {
                        log::error!("{e:#?}");
                        Status::new(520)
                    })?;

                let activity = NewActivity::try_from((
                    ApActivity::Create(self.clone()),
                    Some(ActivityTarget::from(created_question.clone())),
                ) as ApActivityTarget)
                .map_err(|_| Status::new(521))?;

                log::debug!("ACTIVITY\n{activity:#?}");

                if create_activity((&conn).into(), activity).await.is_ok() {
                    runner::run(
                        runner::question::remote_question_task,
                        Some(conn),
                        Some(channels),
                        vec![created_question.ap_id],
                    )
                    .await;
                    Ok(Status::Accepted)
                } else {
                    log::error!("FAILED TO INSERT ACTIVITY");
                    Err(Status::InternalServerError)
                }
            }
            _ => {
                log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
                Err(Status::NotImplemented)
            }
        }
    }
}

impl Outbox for ApCreate {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Profile,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl TryFrom<ExtendedActivity> for ApCreate {
    type Error = &'static str;

    cfg_if::cfg_if! {
        if #[cfg(feature = "pg")] {
            fn try_from(
                (activity, note, _remote_note, _profile, _remote_actor): ExtendedActivity,
            ) -> Result<Self, Self::Error> {
                let note = note.ok_or("ACTIVITY MUST INCLUDE A LOCALLY CREATED NOTE")?;
                let ap_to = activity.ap_to.ok_or("ACTIVITY DOES NOT HAVE A TO FIELD")?;
                Ok(ApCreate {
                    context: Some(ApContext::default()),
                    kind: ApCreateType::default(),
                    actor: ApAddress::Address(activity.actor.clone()),
                    id: Some(format!(
                        "{}/activities/{}",
                        *crate::SERVER_URL,
                        activity.uuid
                    )),
                    object: ApObject::Note(ApNote::from(note)).into(),
                    to: serde_json::from_value(ap_to).unwrap(),
                    cc: activity.cc.map(|cc| serde_json::from_value(cc).unwrap()),
                    signature: None,
                    published: Some(activity.created_at.to_rfc3339()),
                    ephemeral_created_at: Some(activity.created_at),
                    ephemeral_updated_at: Some(activity.updated_at),
                })
            }
        } else if #[cfg(feature = "sqlite")] {
            fn try_from(
                (activity, note, _remote_note, _profile, _remote_actor): ExtendedActivity,
            ) -> Result<Self, Self::Error> {
                let note = note.ok_or("ACTIVITY MUST INCLUDE A LOCALLY CREATED NOTE")?;
                let ap_to = activity.ap_to.ok_or("ACTIVITY DOES NOT HAVE A TO FIELD")?;
                Ok(ApCreate {
                    context: Some(ApContext::default()),
                    kind: ApCreateType::default(),
                    actor: ApAddress::Address(activity.actor.clone()),
                    id: Some(format!(
                        "{}/activities/{}",
                        *crate::SERVER_URL,
                        activity.uuid
                    )),
                    object: ApObject::Note(ApNote::from(note)).into(),
                    to: serde_json::from_str(&ap_to).unwrap(),
                    cc: activity.cc.map(|cc| serde_json::from_str(&cc).unwrap()),
                    signature: None,
                    published: Some(activity.created_at.to_string()),
                    ephemeral_created_at: { Some(DateTime::<Utc>::from_naive_utc_and_offset(
                        activity.created_at,
                        Utc,
                    )) },
                    ephemeral_updated_at: { Some(DateTime::<Utc>::from_naive_utc_and_offset(
                        activity.updated_at,
                        Utc,
                    ))},
                })
            }
        }
    }
}

impl Temporal for ApCreate {
    fn published(&self) -> String {
        if let Some(published) = &self.published {
            published.to_string()
        } else if let MaybeReference::Actual(ApObject::Note(note)) = &self.object {
            note.published.clone()
        } else {
            Utc::now().to_rfc3339()
        }
    }

    fn created_at(&self) -> Option<DateTime<Utc>> {
        self.ephemeral_created_at
    }

    fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.ephemeral_updated_at
    }
}
