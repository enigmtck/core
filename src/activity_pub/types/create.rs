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
        from_serde, from_time,
        profiles::Profile,
        remote_notes::{create_or_update_remote_note, NewRemoteNote},
        remote_questions::create_or_update_remote_question,
    },
    runner, MaybeMultiple, MaybeReference,
};
use anyhow::anyhow;
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
    type Error = anyhow::Error;
    fn try_from(
        (activity, note, _remote_note, _profile, _remote_actor, _remote_question): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        let note = note.ok_or(anyhow!("ACTIVITY MUST INCLUDE A LOCALLY CREATED NOTE"))?;
        let ap_to = activity
            .ap_to
            .ok_or(anyhow!("ACTIVITY DOES NOT HAVE A TO FIELD"))?;
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
            to: from_serde(ap_to).unwrap(),
            cc: activity.cc.and_then(from_serde),
            signature: None,
            published: Some(from_time(activity.created_at).unwrap().to_rfc3339()),
            ephemeral_created_at: from_time(activity.created_at),
            ephemeral_updated_at: from_time(activity.updated_at),
        })
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
