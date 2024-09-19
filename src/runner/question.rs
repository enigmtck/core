use anyhow::Result;

use crate::activity_pub::ApQuestion;
use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::models::profiles::guaranteed_profile;
use crate::models::remote_questions::{get_remote_question_by_ap_id, RemoteQuestion};

use super::actor::get_actor;
use super::TaskError;

pub async fn remote_question_task(
    conn: Option<Db>,
    channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    let ap_id = ap_ids.first().unwrap().clone();

    log::debug!("LOOKING FOR QUESTION AP_ID: {ap_id}");

    if let Some(remote_question) = get_remote_question_by_ap_id(conn, ap_id).await {
        let _ = handle_remote_question(conn, channels, remote_question.clone()).await;
    }

    Ok(())
}

pub async fn handle_remote_question(
    conn: Option<&Db>,
    _channels: Option<EventChannels>,
    remote_question: RemoteQuestion,
) -> anyhow::Result<RemoteQuestion> {
    log::debug!("HANDLING REMOTE QUESTION");

    let question: ApQuestion = remote_question.clone().try_into()?;
    let profile = guaranteed_profile(None, None).await;

    let _ = get_actor(conn, profile, question.attributed_to.to_string()).await;

    Ok(remote_question)
}
