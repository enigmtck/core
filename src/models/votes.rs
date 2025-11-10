use anyhow::Result;
use chrono::{Duration, Utc};
use jdt_activity_pub::{ApCollectionType, ApNote, ApQuestion, MaybeMultiple, QuestionCollection};

use crate::db::runner::DbRunner;
use crate::models::objects::{create_object, get_object_by_as_id, NewObject, Object};

#[derive(Debug)]
pub enum VoteError {
    QuestionNotFound,
    VotingEnded,
    AlreadyVoted,
    InvalidOption,
    DatabaseError(String),
}

impl std::fmt::Display for VoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoteError::QuestionNotFound => write!(f, "Question not found"),
            VoteError::VotingEnded => write!(f, "Voting period has ended"),
            VoteError::AlreadyVoted => write!(f, "Actor has already voted"),
            VoteError::InvalidOption => write!(f, "Invalid vote option"),
            VoteError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for VoteError {}

/// Detect if a Note is a vote by checking if it has a name and inReplyTo
pub fn is_vote(note: &ApNote) -> bool {
    note.name.is_some() && !note.in_reply_to.is_none()
}

/// Get the Question object that this Note is voting on
pub async fn get_question_for_vote<C: DbRunner>(conn: &C, note: &ApNote) -> Result<Option<Object>> {
    // Extract the question ID from inReplyTo using ApNote's get_reply_to method
    let question_id = match note.get_reply_to() {
        Some(id) => id,
        None => return Ok(None),
    };

    // Fetch the Question object
    match get_object_by_as_id(conn, question_id).await {
        Ok(obj) => {
            // Verify it's actually a Question
            if obj.as_type.to_string().to_lowercase() == "question" {
                Ok(Some(obj))
            } else {
                Ok(None)
            }
        }
        Err(_) => Ok(None),
    }
}

/// Check if an actor has already voted on a Question (any option)
pub async fn has_voted<C: DbRunner>(
    conn: &C,
    question_as_id: String,
    actor_as_id: String,
) -> Result<bool> {
    use crate::schema::objects::dsl::*;
    use diesel::prelude::*;

    let count: i64 = conn
        .run(move |c| {
            objects
                .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(&format!(
                    "as_in_reply_to = '\"{}\"'::jsonb",
                    question_as_id
                )))
                .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(&format!(
                    "as_attributed_to = '\"{}\"'::jsonb",
                    actor_as_id
                )))
                .count()
                .get_result(c)
        })
        .await?;

    Ok(count > 0)
}

/// Check if an actor has already voted for a specific option in a Question
pub async fn has_voted_for_option<C: DbRunner>(
    conn: &C,
    question_as_id: String,
    actor_as_id: String,
    option_name: String,
) -> Result<bool> {
    use crate::schema::objects::dsl::*;
    use diesel::prelude::*;

    let count: i64 = conn
        .run(move |c| {
            objects
                .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(&format!(
                    "as_in_reply_to = '\"{}\"'::jsonb",
                    question_as_id
                )))
                .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(&format!(
                    "as_attributed_to = '\"{}\"'::jsonb",
                    actor_as_id
                )))
                .filter(as_name.eq(Some(option_name)))
                .count()
                .get_result(c)
        })
        .await?;

    Ok(count > 0)
}

/// Validate a vote before accepting it
pub async fn validate_vote<C: DbRunner>(
    conn: &C,
    note: &ApNote,
    question: &Object,
    voter_actor_id: String,
) -> Result<(), VoteError> {
    // Check if voting period has ended
    if let Some(end_time) = question.as_end_time {
        if Utc::now() > end_time {
            return Err(VoteError::VotingEnded);
        }
    }

    // Get the vote option
    let vote_option = note.name.clone().ok_or(VoteError::InvalidOption)?;

    // Parse the question to get options and determine type
    let question_ap: ApQuestion = question
        .clone()
        .try_into()
        .map_err(|_| VoteError::QuestionNotFound)?;

    let mut valid_options = Vec::new();
    let is_multiple_choice: bool;

    // Collect options from anyOf (multiple choice)
    if let MaybeMultiple::Multiple(options) = &question_ap.any_of {
        is_multiple_choice = true;
        for option in options {
            valid_options.push(option.name.clone());
        }
    }
    // Collect options from oneOf (single choice)
    else if let MaybeMultiple::Multiple(options) = &question_ap.one_of {
        is_multiple_choice = false;
        for option in options {
            valid_options.push(option.name.clone());
        }
    } else {
        return Err(VoteError::InvalidOption);
    }

    // Validate the option is valid
    if !valid_options.contains(&vote_option) {
        return Err(VoteError::InvalidOption);
    }

    // Check for duplicate votes based on question type
    if is_multiple_choice {
        // anyOf: check if user has voted for THIS SPECIFIC option
        let has_voted_for_this_option =
            has_voted_for_option(conn, question.as_id.clone(), voter_actor_id, vote_option)
                .await
                .map_err(|e| VoteError::DatabaseError(e.to_string()))?;

        if has_voted_for_this_option {
            return Err(VoteError::AlreadyVoted);
        }
    } else {
        // oneOf: check if user has voted for ANY option
        let has_voted_already = has_voted(conn, question.as_id.clone(), voter_actor_id)
            .await
            .map_err(|e| VoteError::DatabaseError(e.to_string()))?;

        if has_voted_already {
            return Err(VoteError::AlreadyVoted);
        }
    }

    Ok(())
}

/// Get all votes for a Question
pub async fn get_votes_for_question<C: DbRunner>(
    conn: &C,
    question_as_id: String,
) -> Result<Vec<Object>> {
    use crate::schema::objects::dsl::*;
    use diesel::prelude::*;

    conn.run(move |c| {
        objects
            .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(&format!(
                "as_in_reply_to = '\"{}\"'::jsonb",
                question_as_id
            )))
            .load::<Object>(c)
    })
    .await
}

/// Recalculate and update Question vote counts based on vote Objects
pub async fn update_question_vote_counts<C: DbRunner>(
    conn: &C,
    question: Object,
) -> Result<Object> {
    // Get all votes for this question
    let votes = get_votes_for_question(conn, question.as_id.clone()).await?;

    // Count distinct voters
    let mut unique_voters = std::collections::HashSet::new();
    let mut vote_counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();

    for vote in votes {
        // Extract voter
        if let Some(attributed_to) = vote.as_attributed_to {
            let voter = attributed_to.to_string();
            unique_voters.insert(voter);
        }

        // Count votes per option
        if let Some(vote_option) = vote.as_name {
            *vote_counts.entry(vote_option).or_insert(0) += 1;
        }
    }

    let voters_count = unique_voters.len() as i32;

    // Parse the question
    let mut question_ap: ApQuestion = question.clone().try_into()?;

    // Update anyOf options with vote counts
    if let MaybeMultiple::Multiple(mut options) = question_ap.any_of.clone() {
        for option in options.iter_mut() {
            let count = *vote_counts.get(&option.name).unwrap_or(&0);
            // Update the replies.totalItems count
            option.replies = Some(QuestionCollection {
                total_items: count,
                kind: Some(ApCollectionType::Collection),
            });
        }
        question_ap.any_of = MaybeMultiple::Multiple(options);
    }

    // Update oneOf options with vote counts
    if let MaybeMultiple::Multiple(mut options) = question_ap.one_of.clone() {
        for option in options.iter_mut() {
            let count = *vote_counts.get(&option.name).unwrap_or(&0);
            // Update the replies.totalItems count
            option.replies = Some(QuestionCollection {
                total_items: count,
                kind: Some(ApCollectionType::Collection),
            });
        }
        question_ap.one_of = MaybeMultiple::Multiple(options);
    }

    // Update voters_count
    question_ap.voters_count = Some(voters_count);

    // Convert back to NewObject
    let mut updated_object: NewObject = question_ap.into();
    updated_object.ek_profile_id = question.ek_profile_id;

    // Update in database
    let updated = create_object(conn, updated_object).await?;

    Ok(updated)
}

/// Check if we should send an Update(Question) activity based on last update time
/// Returns true if:
/// - The Question's updated_at is more than 5 minutes old, OR
/// - We're in the last 5 minutes before endTime (send on every vote)
pub fn should_send_question_update(question: &Object) -> bool {
    let now = Utc::now();
    let last_update = question.updated_at;
    let time_since_update = now - last_update;

    // Check if we're in the final 5 minutes before voting closes
    if let Some(end_time) = question.as_end_time {
        let time_until_end = end_time - now;

        // In the last 5 minutes, send update on every vote (no rate limit)
        if time_until_end <= Duration::minutes(5) && time_until_end > Duration::zero() {
            return true;
        }
    }

    // Normal rate limit: send update if more than 5 minutes have passed since last update
    time_since_update > Duration::minutes(5)
}
