use anyhow::Result;

use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::models::actors::guaranteed_actor;
use crate::models::objects::{get_object_by_as_id, Object};
use crate::retriever::get_actor;
use jdt_activity_pub::ApQuestion;

use super::TaskError;

pub async fn remote_question_task(
    conn: Db,
    channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let ap_id = ap_ids.first().unwrap().clone();

    log::debug!("LOOKING FOR QUESTION AP_ID: {ap_id}");

    if let Ok(remote_question) = get_object_by_as_id(Some(&conn), ap_id).await {
        let _ = handle_remote_question(&conn, channels, remote_question.clone()).await;
    }

    Ok(())
}

pub async fn handle_remote_question(
    conn: &Db,
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
