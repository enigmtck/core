use super::Inbox;

use crate::{
    db::runner::DbRunner,
    models::{
        activities::{create_activity, get_activity_by_ap_id, ActivityTarget, NewActivity},
        objects::{create_object, NewObject},
        unprocessable::create_unprocessable,
        votes::{get_question_for_vote, is_vote, validate_vote, VoteError},
    },
    runner,
    server::AppState,
};
use anyhow::Result;
use jdt_activity_pub::MaybeReference;
use jdt_activity_pub::{ApActivity, ApAddress, ApCreate, ApObject};
use reqwest::StatusCode;
use serde_json::Value;

impl Inbox for ApCreate {
    async fn inbox<C: DbRunner>(
        &self,
        conn: &C,
        state: AppState,
        raw: Value,
    ) -> Result<StatusCode, StatusCode> {
        log::debug!("{:?}", self.clone());

        if let Some(id) = self.id.clone() {
            if get_activity_by_ap_id(conn, id)
                .await
                .is_ok_and(|x| x.is_some())
            {
                return Ok(StatusCode::ACCEPTED);
            }
        }

        log::debug!("Processing Create with object: {:?}", self.object);

        match self.clone().object {
            MaybeReference::Actual(ApObject::Note(x)) => {
                log::debug!("{x:?}");

                // Check if this Note is a vote
                if is_vote(&x) {
                    log::debug!("Detected vote submission");

                    // Get the Question being voted on
                    match get_question_for_vote(conn, &x).await {
                        Ok(Some(question)) => {
                            // Extract voter actor ID from the Note
                            let voter_actor_id = x.attributed_to.to_string();

                            // Validate the vote
                            match validate_vote(conn, &x, &question, voter_actor_id.clone()).await {
                                Ok(()) => {
                                    log::debug!("Vote validated successfully");

                                    // Create the vote Note object
                                    let new_object = NewObject::from(x.clone());
                                    let object =
                                        create_object(conn, new_object).await.map_err(|e| {
                                            log::error!("FAILED TO CREATE VOTE OBJECT: {e:#?}");
                                            StatusCode::INTERNAL_SERVER_ERROR
                                        })?;

                                    // Create the activity
                                    let mut activity = NewActivity::try_from((
                                        ApActivity::Create(self.clone()),
                                        Some(ActivityTarget::from(object.clone())),
                                    ))
                                    .map_err(|e| {
                                        log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                                        StatusCode::INTERNAL_SERVER_ERROR
                                    })?;

                                    activity.raw = Some(raw);

                                    if create_activity(conn, activity).await.is_err() {
                                        log::error!("FAILED TO INSERT ACTIVITY");
                                        return Err(StatusCode::NO_CONTENT);
                                    }

                                    log::debug!("Remote vote recorded");

                                    // Queue background task to update counts and potentially send Update(Question)
                                    let pool_clone = state.db_pool.clone();
                                    let question_id = question.as_id.clone();
                                    runner::run(
                                        runner::question::update_question_vote_counts_task,
                                        pool_clone,
                                        None,
                                        vec![question_id],
                                    )
                                    .await;

                                    Ok(StatusCode::ACCEPTED)
                                }
                                Err(VoteError::VotingEnded) => {
                                    log::warn!("Vote rejected: voting period has ended");
                                    Err(StatusCode::FORBIDDEN)
                                }
                                Err(VoteError::AlreadyVoted) => {
                                    log::warn!("Vote rejected: actor has already voted");
                                    Err(StatusCode::CONFLICT)
                                }
                                Err(VoteError::InvalidOption) => {
                                    log::warn!("Vote rejected: invalid option");
                                    Err(StatusCode::BAD_REQUEST)
                                }
                                Err(e) => {
                                    log::error!("Vote validation failed: {e}");
                                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                                }
                            }
                        }
                        Ok(None) => {
                            log::warn!("Vote submitted but Question not found");
                            Err(StatusCode::NOT_FOUND)
                        }
                        Err(e) => {
                            log::error!("Error getting question for vote: {e:#?}");
                            Err(StatusCode::INTERNAL_SERVER_ERROR)
                        }
                    }
                } else {
                    // Regular Note processing
                    let new_object = NewObject::from(x.clone());

                    let object = create_object(conn, new_object).await.map_err(|e| {
                        log::error!("FAILED TO CREATE OR UPDATE OBJECT: {e:#?}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                    let mut activity = NewActivity::try_from((
                        ApActivity::Create(self.clone()),
                        Some(ActivityTarget::from(object.clone())),
                    ))
                    .map_err(|e| {
                        log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                    activity.raw = Some(raw);

                    if create_activity(conn, activity).await.is_ok() {
                        let pool = state.db_pool.clone();
                        let object_id = object.as_id.clone();

                        runner::run(runner::note::object_task, pool, None, vec![object_id]).await;

                        Ok(StatusCode::ACCEPTED)
                    } else {
                        log::error!("FAILED TO INSERT ACTIVITY");
                        Err(StatusCode::NO_CONTENT)
                    }
                }
            }
            MaybeReference::Actual(ApObject::Article(article)) => {
                log::debug!("{article:?}");
                let new_object = NewObject::from(article.clone());

                let object = create_object(conn, new_object).await.map_err(|e| {
                    log::error!("FAILED TO CREATE OR UPDATE ARTICLE: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

                let mut activity = NewActivity::try_from((
                    ApActivity::Create(self.clone()),
                    Some(ActivityTarget::from(object.clone())),
                ))
                .map_err(|e| {
                    log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

                activity.raw = Some(raw);

                if create_activity(conn, activity).await.is_ok() {
                    let pool = state.db_pool.clone();
                    let object_id = object.as_id.clone();

                    runner::run(runner::note::object_task, pool, None, vec![object_id]).await;

                    Ok(StatusCode::ACCEPTED)
                } else {
                    log::error!("FAILED TO INSERT ACTIVITY");
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
            MaybeReference::Actual(ApObject::Question(question)) => {
                log::debug!("{question:?}");
                let new_object = NewObject::from(question.clone());

                let object = create_object(conn, new_object).await.map_err(|e| {
                    log::error!("FAILED TO CREATE OR UPDATE Object: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

                let mut activity = NewActivity::try_from((
                    ApActivity::Create(self.clone()),
                    Some(ActivityTarget::from(object.clone())),
                ))
                .map_err(|e| {
                    log::error!("FAILED TO BUILD ACTIVITY: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

                activity.raw = Some(raw);

                if create_activity(conn, activity).await.is_ok() {
                    let pool = state.db_pool.clone();
                    let object_id = object.as_id.clone();

                    runner::run(runner::note::object_task, pool, None, vec![object_id]).await;

                    Ok(StatusCode::ACCEPTED)
                } else {
                    log::error!("FAILED TO INSERT ACTIVITY");
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
            _ => {
                log::error!("FAILED TO CREATE ACTIVITY\n{raw}");
                create_unprocessable(
                    conn,
                    (raw, Some("Create object not implemented".to_string())).into(),
                )
                .await;
                Err(StatusCode::NOT_IMPLEMENTED)
            }
        }
    }

    fn actor(&self) -> ApAddress {
        self.actor.clone()
    }
}
