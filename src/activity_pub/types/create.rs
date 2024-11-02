use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{
        ActivityPub, ApActivity, ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox, Temporal,
    },
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::{
            create_activity, ActivityTarget, ActivityType, ApActivityTarget, ExtendedActivity,
            NewActivity,
        },
        actors::Actor,
        from_serde, from_time,
        objects::{create_or_update_object, NewObject},
        pg::coalesced_activity::CoalescedActivity,
    },
    runner, MaybeMultiple, MaybeReference,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{question::ApQuestion, signature::ApSignature};

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

impl TryFrom<String> for ApCreateType {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.to_lowercase() == "create" {
            Ok(ApCreateType::Create)
        } else {
            Err(anyhow!("not a create type"))
        }
    }
}

impl TryFrom<ActivityType> for ApCreateType {
    type Error = anyhow::Error;

    fn try_from(t: ActivityType) -> Result<Self, Self::Error> {
        match t {
            ActivityType::Create => Ok(ApCreateType::Create),
            _ => Err(anyhow!("invalid ActivityType")),
        }
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
                let object = create_or_update_object(&conn, NewObject::from(x.clone()))
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO CREATE OR UPDATE OBJECT: {e:#?}");
                        Status::InternalServerError
                    })?;

                let mut activity = NewActivity::try_from((
                    ApActivity::Create(self.clone()),
                    Some(ActivityTarget::from(object.clone())),
                ))
                .map_err(|e| {
                    log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                    Status::InternalServerError
                })?;

                activity.raw = Some(raw);

                if create_activity((&conn).into(), activity).await.is_ok() {
                    runner::run(
                        runner::note::object_task,
                        Some(conn),
                        Some(channels),
                        vec![object.as_id],
                    )
                    .await;

                    Ok(Status::Accepted)
                } else {
                    log::error!("FAILED TO INSERT ACTIVITY");
                    Err(Status::NoContent)
                }
            }
            MaybeReference::Actual(ApObject::Question(question)) => {
                let object = create_or_update_object(&conn, NewObject::from(question.clone()))
                    .await
                    .map_err(|e| {
                        log::error!("FAILED TO CREATE OR UPDATE Object: {e:#?}");
                        Status::InternalServerError
                    })?;

                let mut activity = NewActivity::try_from((
                    ApActivity::Create(self.clone()),
                    Some(ActivityTarget::from(object.clone())),
                ))
                .map_err(|e| {
                    log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                    Status::InternalServerError
                })?;

                activity.raw = Some(raw);

                if create_activity((&conn).into(), activity).await.is_ok() {
                    runner::run(
                        runner::note::object_task,
                        Some(conn),
                        Some(channels),
                        vec![object.as_id],
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
        _profile: Actor,
        _raw: Value,
    ) -> Result<String, Status> {
        Err(Status::ServiceUnavailable)
    }
}

impl TryFrom<CoalescedActivity> for ApCreate {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        let object = match coalesced
            .clone()
            .object_type
            .ok_or_else(|| anyhow::anyhow!("object_type is None"))?
            .to_string()
            .to_lowercase()
            .as_str()
        {
            "note" => Ok(ApObject::Note(ApNote::try_from(coalesced.clone())?).into()),
            "question" => Ok(ApObject::Question(ApQuestion::try_from(coalesced.clone())?).into()),
            _ => Err(anyhow!("invalid type")),
        }?;
        let kind = coalesced.kind.clone().try_into()?;
        let actor = ApAddress::Address(coalesced.actor.clone());
        let id = coalesced.ap_id.clone();
        let context = Some(ApContext::default());
        let to = coalesced
            .ap_to
            .clone()
            .and_then(from_serde)
            .ok_or_else(|| anyhow::anyhow!("ap_to is None"))?;
        let cc = coalesced.clone().cc.and_then(from_serde);
        let signature = None;
        let published = Some(ActivityPub::time(from_time(coalesced.created_at).unwrap()));
        let ephemeral_created_at = from_time(coalesced.created_at);
        let ephemeral_updated_at = from_time(coalesced.updated_at);

        Ok(ApCreate {
            context,
            kind,
            actor,
            id,
            object,
            to,
            cc,
            signature,
            published,
            ephemeral_created_at,
            ephemeral_updated_at,
        })
    }
}

impl TryFrom<ExtendedActivity> for ApCreate {
    type Error = anyhow::Error;
    fn try_from(
        (activity, _target_activity, target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        let note = {
            if let Some(object) = target_object {
                ApObject::Note(ApNote::try_from(object)?)
            } else {
                return Err(anyhow!("ACTIVITY MUST INCLUDE A NOTE OR REMOTE_NOTE"));
            }
        };

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
            object: note.into(),
            to: from_serde(ap_to).unwrap(),
            cc: activity.cc.and_then(from_serde),
            signature: None,
            published: Some(ActivityPub::time(from_time(activity.created_at).unwrap())),
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
            ActivityPub::time(Utc::now())
        }
    }

    fn created_at(&self) -> Option<DateTime<Utc>> {
        self.ephemeral_created_at
    }

    fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.ephemeral_updated_at
    }
}

impl TryFrom<ApObject> for ApCreate {
    type Error = anyhow::Error;

    fn try_from(object: ApObject) -> Result<ApCreate> {
        match object.clone() {
            ApObject::Note(note) => {
                let context = Some(ApContext::default());
                let kind = ApCreateType::default();
                let actor = note.attributed_to;
                let id = None; // The ID is assigned in NewActivity
                let object = object.into();
                let to = note.to;
                let cc = note.cc;
                let signature = None;
                let published = Some(note.published);
                let ephemeral_created_at = None;
                let ephemeral_updated_at = None;

                Ok(ApCreate {
                    context,
                    kind,
                    actor,
                    id,
                    object,
                    to,
                    cc,
                    signature,
                    published,
                    ephemeral_created_at,
                    ephemeral_updated_at,
                })
            }
            _ => Err(anyhow!("unimplemented object type")),
        }
    }
}
