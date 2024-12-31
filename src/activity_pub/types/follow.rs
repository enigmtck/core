use core::fmt;
use std::fmt::Debug;

use crate::{
    activity_pub::{ApAccept, ApActivity, ApAddress, ApContext, ApObject, Inbox, Outbox},
    db::Db,
    fairings::events::EventChannels,
    helper::get_activity_ap_id_from_uuid,
    models::{
        activities::{
            create_activity, get_activity_by_ap_id, get_activity_by_kind_actor_id_and_target_ap_id,
            ActivityTarget, ActivityType, ExtendedActivity, NewActivity,
        },
        actors::{get_actor, get_actor_by_as_id, Actor},
        coalesced_activity::CoalescedActivity,
        followers::{create_follower, NewFollower},
    },
    routes::ActivityJson,
    runner::{self, get_inboxes, send_to_inboxes, TaskError},
    MaybeMultiple, MaybeReference,
};
use anyhow::anyhow;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ApFollowType {
    #[default]
    #[serde(alias = "follow")]
    Follow,
}

impl fmt::Display for ApFollowType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApFollow {
    #[serde(rename = "@context")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ApContext>,
    #[serde(rename = "type")]
    pub kind: ApFollowType,
    pub actor: ApAddress,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub to: MaybeMultiple<ApAddress>,
    #[serde(skip_serializing_if = "MaybeMultiple::is_none")]
    #[serde(default)]
    pub cc: MaybeMultiple<ApAddress>,
    pub object: MaybeReference<ApObject>,
}

impl Inbox for ApFollow {
    async fn inbox(&self, conn: Db, channels: EventChannels, raw: Value) -> Result<Status, Status> {
        let actor_as_id = self
            .object
            .clone()
            .reference()
            .ok_or(Status::UnprocessableEntity)?;

        if self.id.is_none() {
            log::error!("AP_FOLLOW ID IS NONE");
            return Err(Status::UnprocessableEntity);
        };

        let actor = get_actor_by_as_id(&conn, actor_as_id.clone())
            .await
            .map_err(|e| {
                log::error!("FAILED TO RETRIEVE ACTOR: {e:#?}");
                Status::NotFound
            })?;

        let mut activity = NewActivity::try_from((
            ApActivity::Follow(self.clone()),
            Some(ActivityTarget::from(actor)),
        ))
        .map_err(|e| {
            log::error!("FAILED TO BUILD FOLLOW ACTIVITY: {e:#?}");
            Status::InternalServerError
        })?;

        activity.raw = Some(raw);

        let activity = create_activity((&conn).into(), activity)
            .await
            .map_err(|e| {
                log::error!("FAILED TO CREATE FOLLOW ACTIVITY: {e:#?}");
                Status::InternalServerError
            })?;

        runner::run(
            ApFollow::process,
            conn,
            Some(channels),
            vec![activity.ap_id.ok_or(Status::InternalServerError)?],
        )
        .await;

        Ok(Status::Accepted)
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}

impl Outbox for ApFollow {
    async fn outbox(
        &self,
        conn: Db,
        events: EventChannels,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        ApFollow::outbox(conn, events, self.clone(), profile, raw).await
    }
}

impl ApFollow {
    async fn outbox(
        conn: Db,
        channels: EventChannels,
        follow: ApFollow,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        if let MaybeReference::Reference(as_id) = follow.object.clone() {
            let (activity, actor) = {
                if let Some(activity) = get_activity_by_kind_actor_id_and_target_ap_id(
                    &conn,
                    ActivityType::Follow,
                    profile.id,
                    as_id.clone(),
                )
                .await
                {
                    let actor = get_actor_by_as_id(&conn, as_id.clone())
                        .await
                        .map_err(|e| {
                            log::error!("Failed to retrieve Actor: {e:#?}");
                            Status::NotFound
                        })?;
                    (activity, actor)
                } else {
                    let actor = get_actor_by_as_id(&conn, as_id).await.map_err(|e| {
                        log::error!("Failed to retrieve Actor: {e:#?}");
                        Status::NotFound
                    })?;

                    let mut activity =
                        NewActivity::try_from((follow.into(), Some(actor.clone().into())))
                            .map_err(|e| {
                                log::error!("Failed to build Activity: {e:#?}");
                                Status::InternalServerError
                            })?
                            .link_actor(&conn)
                            .await;

                    activity.raw = Some(raw);

                    (
                        create_activity(Some(&conn), activity.clone())
                            .await
                            .map_err(|e| {
                                log::error!("Failed to create Follow activity: {e:#?}");
                                Status::InternalServerError
                            })?,
                        actor,
                    )
                }
            };

            runner::run(
                ApFollow::send,
                conn,
                Some(channels),
                vec![activity.uuid.clone()],
            )
            .await;

            let activity: ApActivity =
                (activity, None, None, Some(actor))
                    .try_into()
                    .map_err(|e| {
                        log::error!("Failed to build ApActivity: {e:#?}");
                        Status::InternalServerError
                    })?;

            Ok(activity.into())
        } else {
            log::error!("Follow object is not a reference");
            Err(Status::BadRequest)
        }
    }

    async fn send(
        conn: Db,
        _channels: Option<EventChannels>,
        ap_ids: Vec<String>,
    ) -> Result<(), TaskError> {
        for ap_id in ap_ids {
            let (activity, target_activity, target_object, target_actor) =
                get_activity_by_ap_id(&conn, get_activity_ap_id_from_uuid(ap_id.clone()))
                    .await
                    .ok_or(TaskError::TaskFailed)?;

            let sender = get_actor(&conn, activity.actor_id.ok_or(TaskError::TaskFailed)?)
                .await
                .ok_or(TaskError::TaskFailed)?;

            let activity =
                ApActivity::try_from((activity, target_activity, target_object, target_actor))
                    .map_err(|e| {
                        log::error!("FAILED TO BUILD AP_ACTIVITY: {e:#?}");
                        TaskError::TaskFailed
                    })?;

            let inboxes: Vec<ApAddress> =
                get_inboxes(&conn, activity.clone(), sender.clone()).await;

            send_to_inboxes(&conn, inboxes, sender, activity.clone())
                .await
                .map_err(|e| {
                    log::error!("FAILED TO SEND TO INBOXES: {e:#?}");
                    TaskError::TaskFailed
                })?;
        }
        Ok(())
    }

    async fn process(
        conn: Db,
        _channels: Option<EventChannels>,
        ap_ids: Vec<String>,
    ) -> Result<(), TaskError> {
        log::debug!("PROCESSING INCOMING FOLLOW REQUEST");

        for ap_id in ap_ids {
            log::debug!("AS_ID: {ap_id}");

            let extended_follow = get_activity_by_ap_id(&conn, ap_id)
                .await
                .ok_or(TaskError::TaskFailed)?;

            let follow = ApFollow::try_from(extended_follow).map_err(|e| {
                log::error!("FAILED TO BUILD FOLLOW: {e:#?}");
                TaskError::TaskFailed
            })?;
            let accept = ApAccept::try_from(follow.clone()).map_err(|e| {
                log::error!("FAILED TO BUILD ACCEPT: {e:#?}");
                TaskError::TaskFailed
            })?;

            let accept_actor = get_actor_by_as_id(&conn, accept.actor.clone().to_string())
                .await
                .map_err(|e| {
                    log::error!("FAILED TO RETRIEVE ACTOR: {e:#?}");
                    TaskError::TaskFailed
                })?;

            let follow_actor = get_actor_by_as_id(&conn, follow.actor.clone().to_string())
                .await
                .map_err(|e| {
                    log::error!("FAILED TO RETRIEVE ACTOR: {e:#?}");
                    TaskError::TaskFailed
                })?;

            send_to_inboxes(
                &conn,
                vec![follow_actor.as_inbox.clone().into()],
                accept_actor.clone(),
                ApActivity::Accept(Box::new(accept)),
            )
            .await
            .map_err(|e| {
                log::error!("FAILED TO SEND ACCEPT TO INBOXES: {e:#?}");
                TaskError::TaskFailed
            })?;

            let follower = NewFollower::try_from(follow)
                .map_err(|e| {
                    log::error!("FAILED TO BUILD FOLLOWER: {e:#?}");
                    TaskError::TaskFailed
                })?
                .link(accept_actor.clone());

            if create_follower(Some(&conn), follower).await.is_some() {
                log::info!("FOLLOWER CREATED");
            }
        }

        Ok(())
    }
}

impl TryFrom<ExtendedActivity> for ApFollow {
    type Error = anyhow::Error;

    fn try_from(
        (activity, _target_activity, _target_object, _target_actor): ExtendedActivity,
    ) -> Result<Self, Self::Error> {
        if activity.kind.is_follow() {
            let target = activity
                .target_ap_id
                .ok_or(anyhow!("no target_ap_id on follow"))?;
            Ok(ApFollow {
                context: Some(ApContext::default()),
                kind: ApFollowType::default(),
                actor: activity.actor.into(),
                id: Some(activity.ap_id.ok_or(anyhow!("no follow as_id found"))?),
                to: activity.ap_to.into(),
                cc: activity.cc.into(),
                object: target.into(),
            })
        } else {
            log::error!("Not a Follow Activity");
            Err(anyhow!("Not a Follow Activity"))
        }
    }
}

impl TryFrom<CoalescedActivity> for ApFollow {
    type Error = anyhow::Error;

    fn try_from(activity: CoalescedActivity) -> Result<Self, Self::Error> {
        if activity.kind.is_follow() {
            Ok(ApFollow {
                context: Some(ApContext::default()),
                kind: ApFollowType::default(),
                actor: activity.actor.into(),
                id: Some(activity.ap_id.ok_or(anyhow!("no follow as_id found"))?),
                to: activity.ap_to.into(),
                cc: activity.cc.into(),
                object: activity
                    .object_as_id
                    .ok_or(anyhow!("no object_as_id"))?
                    .into(),
            })
        } else {
            log::error!("Not a Follow Activity");
            Err(anyhow!("Not a Follow Activity"))
        }
    }
}
