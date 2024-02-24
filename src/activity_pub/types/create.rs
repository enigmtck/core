use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox, Temporal},
    db::Db,
    fairings::{events::EventChannels, faktory::FaktoryConnection},
    models::{
        activities::{
            create_activity, ActivityTarget, ApActivityTarget, ExtendedActivity, NewActivity,
        },
        profiles::Profile,
        remote_notes::{create_or_update_remote_note, NewRemoteNote},
    },
    to_faktory, MaybeMultiple, MaybeReference,
};
use chrono::{DateTime, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::signature::ApSignature;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApCreateType {
    #[default]
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
    async fn inbox(
        &self,
        conn: Db,
        faktory: FaktoryConnection,
        raw: Value,
    ) -> Result<Status, Status> {
        //inbox::activity::create(conn, faktory, self.clone()).await
        match self.clone().object {
            MaybeReference::Actual(ApObject::Note(x)) => {
                let n = NewRemoteNote::from(x.clone());

                // creating Activity after RemoteNote is weird, but currently necessary
                // see comment in models/activities.rs on TryFrom<ApActivity>
                if let Some(created_note) = create_or_update_remote_note(&conn, n).await {
                    if let Ok(activity) = NewActivity::try_from((
                        ApActivity::Create(self.clone()),
                        Some(ActivityTarget::from(created_note.clone())),
                    )
                        as ApActivityTarget)
                    {
                        log::debug!("ACTIVITY\n{activity:#?}");
                        if create_activity((&conn).into(), activity).await.is_some() {
                            to_faktory(faktory, "process_remote_note", created_note.ap_id)
                        } else {
                            log::error!("FAILED TO INSERT ACTIVITY");
                            Err(Status::NoContent)
                        }
                    } else {
                        log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
                        Err(Status::NoContent)
                    }
                } else {
                    log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
                    Err(Status::NoContent)
                }
            }
            _ => {
                log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
                Err(Status::NoContent)
            }
        }
    }
}

impl Outbox for ApCreate {
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

impl TryFrom<ExtendedActivity> for ApCreate {
    type Error = &'static str;

    fn try_from(
        (activity, note, _remote_note, _profile, _remote_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if let Some(note) = note {
            if let Some(ap_to) = activity.ap_to {
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
            } else {
                log::error!("ACTIVITY DOES NOT HAVE A TO FIELD");
                Err("ACTIVITY DOES NOT HAVE A TO FIELD")
            }
        } else {
            log::error!("ACTIVITY MUST INCLUDE A LOCALLY CREATED NOTE");
            Err("ACTIVITY MUST INCLUDE A LOCALLY CREATED NOTE")
        }
    }
}

// impl From<ApNote> for ApCreate {
//     fn from(note: ApNote) -> Self {
//         ApCreate {
//             context: Some(ApContext::default()),
//             kind: ApCreateType::default(),
//             actor: note.attributed_to.clone(),
//             id: note.id.clone().map(|id| format!("{id}#create")),
//             object: ApObject::Note(note.clone()).into(),
//             to: note.to.clone(),
//             cc: note.cc.clone(),
//             signature: None,
//             published: note.published,
//             ephemeral_created_at: None,
//             ephemeral_updated_at: None,
//         }
//     }
// }

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
