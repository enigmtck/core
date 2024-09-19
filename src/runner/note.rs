use anyhow::Result;
use reqwest::StatusCode;
use webpage::{Webpage, WebpageOptions};

use crate::activity_pub::retriever::signed_get;
use crate::activity_pub::{ApAttachment, ApImage, Metadata};
use crate::fairings::events::EventChannels;
use crate::models::cache::{Cache, Cacheable};
use crate::models::note_hashtags::{create_note_hashtag, NewNoteHashtag};
use crate::models::notes::delete_note_by_uuid;
use crate::models::pg::remote_notes::update_metadata;
use crate::models::profiles::{get_profile, guaranteed_profile};
use crate::models::remote_note_hashtags::{create_remote_note_hashtag, NewRemoteNoteHashtag};
use crate::models::remote_notes::{create_or_update_remote_note, get_remote_note_by_ap_id};
use crate::runner::cache::cache_content;
use crate::ANCHOR_RE;
use crate::{
    activity_pub::{ApActivity, ApAddress, ApNote, ApObject},
    db::Db,
    models::{activities::get_activity_by_uuid, profiles::Profile, remote_notes::RemoteNote},
    runner::{
        //encrypted::handle_encrypted_note,
        get_inboxes,
        send_to_inboxes,
    },
    signing::Method,
    MaybeReference,
};

use super::actor::get_actor;
use super::TaskError;

pub async fn delete_note_task(
    conn: Option<Db>,
    _channels: Option<EventChannels>,
    uuids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    for uuid in uuids {
        log::debug!("LOOKING FOR UUID {uuid}");

        let (
            activity,
            target_note,
            target_remote_note,
            target_profile,
            target_remote_actor,
            target_remote_question,
            target_remote_note_hashtag,
        ) = get_activity_by_uuid(conn, uuid.clone())
            .await
            .ok_or(TaskError::TaskFailed)?;

        log::debug!("FOUND ACTIVITY\n{activity:#?}");
        let profile_id = activity.profile_id.ok_or(TaskError::TaskFailed)?;
        let sender = get_profile(conn, profile_id)
            .await
            .ok_or(TaskError::TaskFailed)?;
        let note = target_note.clone().ok_or(TaskError::TaskFailed)?;

        let activity = ApActivity::try_from((
            (
                activity,
                target_note,
                target_remote_note,
                target_profile,
                target_remote_actor,
                target_remote_question,
                target_remote_note_hashtag,
            ),
            None,
        ))
        .map_err(|_| TaskError::TaskFailed)?;
        let inboxes: Vec<ApAddress> = get_inboxes(conn, activity.clone(), sender.clone()).await;

        send_to_inboxes(inboxes, sender, activity.clone())
            .await
            .map_err(|_| TaskError::TaskFailed)?;

        let records = delete_note_by_uuid(conn, note.uuid)
            .await
            .map_err(|_| TaskError::TaskFailed)?;
        log::debug!("NOTE RECORDS DELETED: {records}");
    }

    Ok(())
}

pub async fn outbound_note_task(
    conn: Option<Db>,
    _channels: Option<EventChannels>,
    uuids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    for uuid in uuids {
        let (
            activity,
            target_note,
            target_remote_note,
            target_profile,
            target_remote_actor,
            target_remote_question,
            target_remote_note_hashtag,
        ) = get_activity_by_uuid(conn, uuid)
            .await
            .ok_or(TaskError::TaskFailed)?;

        let profile_id = activity.profile_id.ok_or(TaskError::TaskFailed)?;

        let sender = get_profile(None, profile_id)
            .await
            .ok_or(TaskError::TaskFailed)?;
        let note = target_note.clone().ok_or(TaskError::TaskFailed)?;

        let new_tags: Vec<NewNoteHashtag> = note.clone().into();

        let _ = new_tags
            .iter()
            .map(|tag| async { create_note_hashtag(None, tag.clone()).await });

        // For the Svelte client, all images are passed through the server cache. To cache an image
        // that's already on the server seems weird, but I think it's a better choice than trying
        // to handle the URLs for local images differently.
        let ap_note: ApNote = note.clone().into();
        if let Some(attachments) = ap_note.attachment {
            for attachment in attachments {
                if let ApAttachment::Document(document) = attachment {
                    let _ = cache_content(Cacheable::try_from(ApImage::try_from(document))).await;
                }
            }
        }

        cfg_if::cfg_if! {
            if #[cfg(feature = "pg")] {
                use crate::models::notes::NoteType;

                let activity = match note.kind {
                    NoteType::Note => {
                        if let Ok(activity) = ApActivity::try_from((
                            (
                                activity,
                                target_note,
                                target_remote_note,
                                target_profile,
                                target_remote_actor,
                                target_remote_question,
                                target_remote_note_hashtag,
                            ),
                            None,
                        )) {
                            Some(activity)
                        } else {
                            None
                        }
                    }
                    // NoteType::EncryptedNote => {
                    //     handle_encrypted_note(&mut note, sender.clone())
                    //         .map(ApActivity::Create(ApCreate::from))
                    // }
                    _ => None,
                };
            } else if #[cfg(feature = "sqlite")] {
                let activity = {
                    if note.kind.to_lowercase().as_str() == "note" {
                        if let Ok(activity) = ApActivity::try_from((
                            (
                                activity,
                                target_note,
                                target_remote_note,
                                target_profile,
                                target_remote_actor,
                                target_remote_question,
                                target_remote_note_hashtag,
                            ),
                            None,
                        )) {
                            Some(activity)
                        } else {
                            None
                        }
                    } else {
                        // NoteType::EncryptedNote => {
                        //     handle_encrypted_note(&mut note, sender.clone())
                        //         .map(ApActivity::Create(ApCreate::from))
                        // }
                        None
                    }
                };
            }
        }

        let _ = get_actor(conn, sender.clone(), note.clone().attributed_to).await;

        let activity = activity.ok_or(TaskError::TaskFailed)?;

        let inboxes: Vec<ApAddress> = get_inboxes(conn, activity.clone(), sender.clone()).await;

        log::debug!("SENDING ACTIVITY\n{activity:#?}");
        log::debug!("INBOXES\n{inboxes:#?}");

        send_to_inboxes(inboxes, sender, activity)
            .await
            .map_err(|_| TaskError::TaskFailed)?;
    }

    Ok(())
}

pub async fn retrieve_context_task(
    conn: Option<Db>,
    _channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    let profile = guaranteed_profile(None, None).await;

    for ap_id in ap_ids {
        let profile = profile.clone();

        if let Some(note) = fetch_remote_note(
            conn.ok_or(TaskError::TaskFailed)?,
            ap_id.to_string(),
            profile,
        )
        .await
        {
            log::debug!("REPLIES\n{:#?}", note.replies);

            if let Some(replies) = ApNote::from(note).replies {
                if let Some(MaybeReference::Actual(_first)) = replies.first {}
            }
        }
    }

    Ok(())
}

pub async fn fetch_remote_note(conn: &Db, id: String, profile: Profile) -> Option<RemoteNote> {
    log::debug!("PERFORMING REMOTE LOOKUP FOR NOTE: {id}");

    let _url = id.clone();
    let _method = Method::Get;

    if let Ok(resp) = signed_get(profile, id, false).await {
        match resp.status() {
            StatusCode::ACCEPTED | StatusCode::OK => match resp.json().await {
                Ok(ApObject::Note(note)) => {
                    create_or_update_remote_note(Some(conn), note.cache(conn).await.clone().into())
                        .await
                }
                Err(e) => {
                    log::error!("FAILED TO DECODE REMOTE NOTE\n{e:#?}");
                    None
                }
                _ => None,
            },
            StatusCode::GONE => {
                log::debug!("REMOTE NOTE NO LONGER EXISTS AT SOURCE");
                None
            }
            _ => {
                log::error!("REMOTE NOTE FETCH STATUS {:#?}", resp.status());
                log::error!("{:#?}", resp.text().await);
                None
            }
        }
    } else {
        None
    }
}

pub async fn create_remote_note_tags(conn: Option<&Db>, remote_note: RemoteNote) {
    let new_tags: Vec<NewRemoteNoteHashtag> = remote_note.clone().into();

    for tag in new_tags.iter() {
        log::debug!("ADDING HASHTAG: {}", tag.hashtag);
        create_remote_note_hashtag(conn, tag.clone()).await;
    }
}

// TODO: This is problematic for links that point to large files; the filter tries
// to account for some of that, but that's not really a solution. Maybe a whitelist?
// That would suck. I wish the Webpage crate had a size limit (i.e., load pages with
// a maximum size of 10MB or whatever a reasonable amount would be).
fn get_links(text: String) -> Vec<String> {
    ANCHOR_RE
        .captures_iter(&text)
        .filter(|cap| {
            !cap[0].to_lowercase().contains("mention")
                && !cap[0].to_lowercase().contains("u-url")
                && !cap[0].to_lowercase().contains("hashtag")
                && !cap[0].to_lowercase().contains("download")
                && !cap[1].to_lowercase().contains(".pdf")
        })
        .map(|cap| cap[1].to_string())
        .collect()
}

fn metadata(remote_note: &RemoteNote) -> Vec<Metadata> {
    get_links(remote_note.content.clone())
        .iter()
        .map(|link| {
            (
                link.clone(),
                Webpage::from_url(link, WebpageOptions::default()),
            )
        })
        .filter(|(_, metadata)| metadata.is_ok())
        .map(|(link, metadata)| (link, metadata.unwrap().html.meta).into())
        .collect()
}

pub async fn handle_remote_note(
    conn: Option<&Db>,
    channels: Option<EventChannels>,
    mut remote_note: RemoteNote,
    announcer: Option<String>,
) -> anyhow::Result<RemoteNote> {
    log::debug!("HANDLING REMOTE NOTE");

    let metadata = metadata(&remote_note);

    if !metadata.is_empty() {
        remote_note = update_metadata(
            conn,
            remote_note.id,
            serde_json::to_value(metadata).unwrap(),
        )
        .await?;
    }

    let mut note: ApNote = remote_note.clone().into();
    let profile = guaranteed_profile(None, None);

    let _ = get_actor(conn, profile.await, note.attributed_to.to_string()).await;

    if let Some(announcer) = announcer {
        note.ephemeral_announces = Some(vec![announcer]);
    }

    create_remote_note_tags(conn, remote_note.clone()).await;
    note.cache(conn.unwrap()).await;

    if let Some(mut channels) = channels {
        channels.send(None, serde_json::to_string(&note.clone()).unwrap());
    }

    Ok(remote_note)
}

pub async fn handle_remote_encrypted_note_task(
    _conn: Option<&Db>,
    remote_note: RemoteNote,
) -> Result<()> {
    log::debug!("adding to processing queue");

    if let Some(ap_to) = remote_note.clone().ap_to {
        cfg_if::cfg_if! {
            if #[cfg(feature = "pg")] {
                let _to_vec: Vec<String> = serde_json::from_value(ap_to)?;
            } else if #[cfg(feature = "sqlite")] {
                let _to_vec: Vec<String> = serde_json::from_str(&ap_to)?;
            }
        }

        // need to refactor this because of the async in the closures
        // to_vec
        //     .iter()
        //     .filter_map(|ap_id| get_profile_by_ap_id(conn, ap_id.to_string()).await)
        //     .for_each(|profile| {
        //         create_processing_item(None, (remote_note.clone(), profile.id).into()).await;
        //     });
    }

    Ok(())
}

pub async fn remote_note_task(
    conn: Option<Db>,
    channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    let ap_id = ap_ids.first().unwrap().clone();

    log::debug!("looking for ap_id: {}", ap_id);

    if let Some(remote_note) = get_remote_note_by_ap_id(conn, ap_id).await {
        cfg_if::cfg_if! {
            if #[cfg(feature = "pg")] {
                use crate::models::notes::NoteType;

                match remote_note.kind {
                    NoteType::Note => {
                        let _ = handle_remote_note(conn, channels, remote_note.clone(), None).await;
                    }
                    NoteType::EncryptedNote => {
                        let _ = handle_remote_encrypted_note_task(conn, remote_note).await;
                    }
                    _ => (),
                }
            } else if #[cfg(feature = "sqlite")] {
                match remote_note.kind.as_str() {
                    "note" => {
                        let _ = handle_remote_note(conn, channels.clone(), remote_note.clone(), None).await;
                    }
                    "encrypted_note" => {
                        let _ = handle_remote_encrypted_note_task(conn, remote_note).await;
                    }
                    _ => (),
                }
            }
        }
    }

    Ok(())
}
