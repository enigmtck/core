use anyhow::Result;
use deadpool_diesel::postgres::Pool;

use crate::db::runner::DbRunner;
use crate::events::EventChannels;
use crate::models::actors::guaranteed_actor;
use crate::models::objects::{get_object_by_as_id, Object};
use crate::models::votes::{should_send_question_update, update_question_vote_counts};
use crate::retriever::get_actor;
use jdt_activity_pub::{ApActivity, ApAddress, ApObject, ApQuestion, ApUpdate};

use super::TaskError;

pub async fn remote_question_task<C: DbRunner>(
    conn: &C,
    channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let ap_id = ap_ids.first().unwrap().clone();

    log::debug!("LOOKING FOR QUESTION AP_ID: {ap_id}");

    if let Ok(remote_question) = get_object_by_as_id(conn, ap_id).await {
        let _ = handle_remote_question(conn, channels, remote_question.clone()).await;
    }

    Ok(())
}

pub async fn handle_remote_question<C: DbRunner>(
    conn: &C,
    _channels: Option<EventChannels>,
    object: Object,
) -> anyhow::Result<Object> {
    log::debug!("HANDLING REMOTE QUESTION");

    let question: ApQuestion = object.clone().try_into()?;
    let profile = guaranteed_actor(conn, None).await;

    let _ = get_actor(
        conn,
        question.attributed_to.to_string(),
        Some(profile),
        true,
    )
    .await;

    Ok(object)
}

/// Background task to send Update(Question) activity to followers with updated vote counts
pub async fn send_question_update_task(
    pool: Pool,
    _channels: Option<EventChannels>,
    question_ids: Vec<String>,
) -> Result<(), TaskError> {
    for question_id in question_ids {
        let conn = pool.get().await.map_err(|_| TaskError::TaskFailed)?;

        // Get the Question object from database
        let question_object = get_object_by_as_id(&conn, question_id.clone())
            .await
            .map_err(|e| {
                log::error!("Failed to retrieve Question object: {e}");
                TaskError::TaskFailed
            })?;

        log::debug!("Sending update for Question: {:?}", question_object.as_name);

        // Convert to ApQuestion
        let question = ApQuestion::try_from(question_object.clone()).map_err(|e| {
            log::error!("Failed to build ApQuestion: {e:#?}");
            TaskError::TaskFailed
        })?;

        // Get the Question author (who created the Question)
        let profile_id = question_object.ek_profile_id.ok_or_else(|| {
            log::error!("Question ek_profile_id cannot be None");
            TaskError::TaskFailed
        })?;

        let sender = crate::models::actors::get_actor(&conn, profile_id)
            .await
            .map_err(|e| {
                log::error!("Failed to retrieve Question author: {e}");
                TaskError::TaskFailed
            })?;

        // Build Update activity wrapping the Question
        let update = ApUpdate {
            actor: sender.as_id.clone().into(),
            object: ApObject::Question(question).into(),
            ..Default::default()
        };

        let activity = ApActivity::Update(update);

        // Get follower inboxes
        let inboxes: Vec<ApAddress> =
            super::get_inboxes(&conn, activity.clone(), sender.clone()).await;

        log::debug!("SENDING UPDATE(QUESTION) ACTIVITY");
        log::debug!("Question ID: {}", question_id);
        log::debug!("INBOXES: {:#?}", inboxes);

        // Send Update(Question) to followers
        super::send_to_inboxes(&conn, inboxes, sender, activity)
            .await
            .map_err(|e| {
                log::error!("Failed to send Update(Question): {e:#?}");
                TaskError::TaskFailed
            })?;
    }

    Ok(())
}

/// Background task to update Question vote counts and optionally send Update(Question)
/// This is called after a vote is recorded to update counts asynchronously
pub async fn update_question_vote_counts_task(
    pool: Pool,
    _channels: Option<EventChannels>,
    question_ids: Vec<String>,
) -> Result<(), TaskError> {
    for question_id in question_ids {
        let conn = pool.get().await.map_err(|_| TaskError::TaskFailed)?;

        // Get the Question object from database
        let question_object = get_object_by_as_id(&conn, question_id.clone())
            .await
            .map_err(|e| {
                log::error!("Failed to retrieve Question object for count update: {e}");
                TaskError::TaskFailed
            })?;

        // Check if we should send Update(Question) based on rate limit (before updating counts)
        let should_send_update = should_send_question_update(&question_object);

        // Update the Question's vote counts (this will update the updated_at timestamp)
        let updated_question = update_question_vote_counts(&conn, question_object)
            .await
            .map_err(|e| {
                log::error!("Failed to update question vote counts in background task: {e:#?}");
                TaskError::TaskFailed
            })?;

        log::debug!("Question vote counts updated in background");

        // Send Update(Question) if rate limit allows (or in final 5 minutes)
        if should_send_update {
            log::debug!("Sending Update(Question) to followers");
            // Call send_question_update_task directly (we're already in a background task)
            send_question_update_task(pool.clone(), None, vec![updated_question.as_id]).await?;
        } else {
            log::debug!("Skipping Update(Question) - rate limited");
        }
    }

    Ok(())
}
