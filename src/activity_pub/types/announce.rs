use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{
        ActivityPub, ApActivity, ApAddress, ApContext, ApNote, ApObject, Inbox, Outbox, Temporal,
    },
    db::Db,
    fairings::events::EventChannels,
    models::{
        activities::{create_activity, ActivityType, ExtendedActivity, NewActivity},
        actors::Actor,
        coalesced_activity::CoalescedActivity,
        from_serde,
        objects::get_object_by_as_id,
    },
    routes::ActivityJson,
    runner, MaybeMultiple, MaybeReference,
};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::question::ApQuestion;
use super::Ephemeral;

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

impl TryFrom<ActivityType> for ApAnnounceType {
    type Error = anyhow::Error;

    fn try_from(t: ActivityType) -> Result<Self, Self::Error> {
        match t {
            ActivityType::Announce => Ok(ApAnnounceType::Announce),
            _ => Err(anyhow!("invalid ActivityType")),
        }
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
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub cc: MaybeMultiple<ApAddress>,
    pub published: String,
    pub object: MaybeReference<ApObject>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<Ephemeral>,
}

impl Inbox for ApAnnounce {
    async fn inbox(&self, conn: Db, channels: EventChannels, raw: Value) -> Result<Status, Status> {
        let mut activity = NewActivity::try_from((ApActivity::Announce(self.clone()), None))
            .map_err(|e| {
                log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                Status::InternalServerError
            })?;

        activity.raw = Some(raw.clone());

        if create_activity((&conn).into(), activity.clone())
            .await
            .is_ok()
        {
            runner::run(
                runner::announce::remote_announce_task,
                conn,
                Some(channels),
                vec![activity.ap_id.clone().ok_or(Status::BadRequest)?],
            )
            .await;
            Ok(Status::Accepted)
        } else {
            log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
            Err(Status::new(521))
        }
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}

impl Outbox for ApAnnounce {
    async fn outbox(
        &self,
        conn: Db,
        events: EventChannels,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        outbox(conn, events, self.clone(), profile, raw).await
    }
}

async fn outbox(
    conn: Db,
    channels: EventChannels,
    announce: ApAnnounce,
    _profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, Status> {
    if let MaybeReference::Reference(as_id) = announce.clone().object {
        let object = get_object_by_as_id(Some(&conn), as_id).await.map_err(|e| {
            log::error!("FAILED TO RETRIEVE Object: {e:#?}");
            Status::NotFound
        })?;

        let mut activity = NewActivity::try_from((announce.into(), Some(object.clone().into())))
            .map_err(|e| {
                log::error!("FAILED TO BUILD Activity: {e:#?}");
                Status::InternalServerError
            })?
            .link_actor(&conn)
            .await;

        activity.raw = Some(raw);

        let activity = create_activity(Some(&conn), activity).await.map_err(|e| {
            log::error!("FAILED TO CREATE Activity: {e:#?}");
            Status::InternalServerError
        })?;

        runner::run(
            runner::announce::send_announce_task,
            conn,
            Some(channels),
            vec![activity.ap_id.clone().ok_or(Status::InternalServerError)?],
        )
        .await;

        let activity: ApActivity =
            (activity, None, Some(object), None)
                .try_into()
                .map_err(|e| {
                    log::error!("Failed to build ApActivity: {e:#?}");
                    Status::InternalServerError
                })?;

        Ok(activity.into())
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
        self.ephemeral.clone().and_then(|x| x.created_at)
    }

    fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.ephemeral.clone().and_then(|x| x.updated_at)
    }
}

impl TryFrom<CoalescedActivity> for ApAnnounce {
    type Error = anyhow::Error;

    fn try_from(coalesced: CoalescedActivity) -> Result<Self, Self::Error> {
        let object = match coalesced
            .clone()
            .object_type
            .ok_or(anyhow!("object_type is None"))?
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
            .ok_or(anyhow!("ap_to is None"))?;
        let cc = coalesced.clone().cc.into();
        let published = ActivityPub::time(coalesced.created_at);
        let ephemeral = Some(Ephemeral {
            created_at: Some(coalesced.created_at),
            updated_at: Some(coalesced.updated_at),
            ..Default::default()
        });

        Ok(ApAnnounce {
            context,
            kind,
            actor,
            id,
            object,
            to,
            cc,
            published,
            ephemeral,
        })
    }
}

impl TryFrom<ExtendedActivity> for ApAnnounce {
    type Error = anyhow::Error;

    fn try_from(
        (activity, _target_activity, target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if activity.kind.to_string().to_lowercase().as_str() == "announce" {
            let ap_to = activity.ap_to.ok_or(anyhow!("ap_to is None"))?;

            let object = target_object.ok_or(anyhow!("INVALID ACTIVITY TYPE"))?;
            let object = MaybeReference::Actual(ApObject::Note(ApNote::try_from(object)?));

            Ok(ApAnnounce {
                context: Some(ApContext::default()),
                kind: ApAnnounceType::default(),
                actor: activity.actor.into(),
                id: Some(format!(
                    "{}/activities/{}",
                    *crate::SERVER_URL,
                    activity.uuid
                )),
                to: from_serde(ap_to).unwrap(),
                cc: activity.cc.into(),
                published: ActivityPub::time(activity.created_at),
                object,
                ephemeral: Some(Ephemeral {
                    created_at: Some(activity.created_at),
                    updated_at: Some(activity.updated_at),
                    ..Default::default()
                }),
            })
        } else {
            log::error!("NOT AN ANNOUNCE ACTIVITY");
            Err(anyhow::Error::msg("NOT AN ANNOUNCE ACTIVITY"))
        }
    }
}
