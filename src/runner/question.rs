use anyhow::Result;
use reqwest::StatusCode;

use crate::activity_pub::retriever::signed_get;
use crate::activity_pub::{ApAttachment, ApImage, ApQuestion};
use crate::fairings::events::EventChannels;
use crate::models::cache::{Cache, Cacheable};
use crate::models::note_hashtags::{create_note_hashtag, NewNoteHashtag};
use crate::models::notes::delete_note_by_uuid;
use crate::models::profiles::{get_profile, guaranteed_profile};
use crate::models::remote_note_hashtags::{create_remote_note_hashtag, NewRemoteNoteHashtag};
use crate::models::remote_notes::{create_or_update_remote_note, get_remote_note_by_ap_id};
use crate::models::remote_questions::{get_remote_question_by_ap_id, RemoteQuestion};
use crate::models::timeline::{create_timeline_item, delete_timeline_item_by_ap_id, TimelineItem};
use crate::models::timeline_hashtags::{create_timeline_hashtag, NewTimelineHashtag};
use crate::runner::cache::cache_content;
use crate::runner::note::create_timeline_tags;
use crate::{
    activity_pub::{ApActivity, ApAddress, ApNote, ApObject},
    db::Db,
    helper::get_note_ap_id_from_uuid,
    models::{
        activities::get_activity_by_uuid, notes::Note, profiles::Profile, remote_notes::RemoteNote,
    },
    runner::{
        //encrypted::handle_encrypted_note,
        get_inboxes,
        send_to_inboxes,
    },
    signing::Method,
    MaybeReference,
};

use super::TaskError;
use super::{actor::get_actor, timeline::add_to_timeline};

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
    channels: Option<EventChannels>,
    remote_question: RemoteQuestion,
) -> anyhow::Result<RemoteQuestion> {
    log::debug!("HANDLING REMOTE QUESTION");

    let question: ApQuestion = remote_question.clone().try_into()?;
    let profile = guaranteed_profile(None, None).await;

    let _ = get_actor(conn, profile, question.attributed_to.to_string()).await;

    if let Ok(timeline_item) = create_timeline_item(conn, remote_question.clone().into()).await {
        create_timeline_tags(conn, timeline_item.clone()).await;

        add_to_timeline(
            remote_question.clone().ap_to,
            remote_question.clone().cc,
            timeline_item,
        )
        .await;
    }

    Ok(remote_question)
}
