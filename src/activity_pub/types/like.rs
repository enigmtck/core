use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApActivity, ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox},
    db::Db,
    fairings::events::EventChannels,
    helper::get_activity_ap_id_from_uuid,
    models::{
        activities::{
            create_activity, ActivityTarget, ApActivityTarget, ExtendedActivity, NewActivity,
        },
        actors::Actor,
        objects::get_object_by_as_id,
    },
    runner, MaybeMultiple, MaybeReference,
};
use anyhow::anyhow;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApLikeType {
    #[default]
    #[serde(alias = "like")]
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
        _channels: EventChannels,
        raw: Value,
    ) -> Result<Status, Status> {
        let note_apid = match self.object.clone() {
            MaybeReference::Reference(reference) => Some(reference),
            MaybeReference::Actual(ApObject::Note(actual)) => actual.id,
            _ => None,
        };

        if let Some(note_apid) = note_apid {
            log::debug!("NOTE AP_ID\n{note_apid:#?}");
            if let Ok(target) = get_object_by_as_id(Some(&conn), note_apid).await {
                log::debug!("TARGET\n{target:#?}");
                if let Ok(mut activity) = NewActivity::try_from((
                    ApActivity::Like(self.clone()),
                    Some(ActivityTarget::from(target)),
                )
                    as ApActivityTarget)
                {
                    log::debug!("ACTIVITY\n{activity:#?}");
                    activity.raw = Some(raw.clone());

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
        events: EventChannels,
        profile: Actor,
        raw: Value,
    ) -> Result<String, Status> {
        handle_like_outbox(conn, events, *self.clone(), profile).await
    }
}

async fn handle_like_outbox(
    conn: Db,
    channels: EventChannels,
    like: ApLike,
    _profile: Actor,
) -> Result<String, Status> {
    if let MaybeReference::Reference(as_id) = like.clone().object {
        let object = get_object_by_as_id(Some(&conn), as_id)
            .await
            .map_err(|_| Status::NotFound)?;

        if let Ok(activity) = create_activity(
            Some(&conn),
            NewActivity::try_from((Box::new(like).into(), Some(object.into())))
                .map_err(|_| Status::InternalServerError)?
                .link_actor(&conn)
                .await,
        )
        .await
        {
            runner::run(
                runner::like::send_like_task,
                Some(conn),
                Some(channels),
                vec![activity.uuid.clone()],
            )
            .await;
            Ok(get_activity_ap_id_from_uuid(activity.uuid))
        } else {
            log::error!("FAILED TO CREATE LIKE ACTIVITY");
            Err(Status::NoContent)
        }
    } else {
        log::error!("LIKE OBJECT IS NOT A REFERENCE");
        Err(Status::NoContent)
    }
}

impl TryFrom<ExtendedActivity> for ApLike {
    type Error = anyhow::Error;

    fn try_from(
        (activity, _target_activity, target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if !activity.kind.is_like() {
            return Err(anyhow!("NOT A LIKE ACTIVITY"));
        }

        let object = target_object.ok_or(anyhow!("no target object"))?;
        let note = ApNote::try_from(object)?;

        let (id, object): (String, MaybeReference<ApObject>) = (
            note.attributed_to.clone().to_string(),
            MaybeReference::Reference(note.id.ok_or(anyhow!("no note id"))?),
        );

        Ok(ApLike {
            context: Some(ApContext::default()),
            kind: ApLikeType::default(),
            actor: activity.actor.into(),
            id: Some(format!(
                "{}/activities/{}",
                *crate::SERVER_URL,
                activity.uuid
            )),
            to: Some(MaybeMultiple::Single(ApAddress::Address(id))),
            object,
        })
    }
}
