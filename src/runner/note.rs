use anyhow::Result;
use faktory::Job;
use reqwest::{Client, StatusCode};
use rocket::futures::future::join_all;
use tokio::io;
use tokio::runtime::Runtime;
use url::Url;

use crate::activity_pub::retriever::signed_get;
use crate::activity_pub::ApImage;
use crate::fairings::events::EventChannels;
use crate::models::note_hashtags::{create_note_hashtag, NewNoteHashtag};
use crate::models::notes::delete_note_by_uuid;
use crate::models::processing_queue::create_processing_item;
use crate::models::profiles::{get_profile, get_profile_by_ap_id, guaranteed_profile};
use crate::models::remote_note_hashtags::{create_remote_note_hashtag, NewRemoteNoteHashtag};
use crate::models::remote_notes::{create_or_update_remote_note, get_remote_note_by_ap_id};
use crate::models::timeline::{create_timeline_item, delete_timeline_item_by_ap_id, TimelineItem};
use crate::models::timeline_hashtags::{create_timeline_hashtag, NewTimelineHashtag};
use crate::runner::cache::cache_content;
use crate::{
    activity_pub::{ApActivity, ApAddress, ApNote, ApObject},
    db::Db,
    helper::get_note_ap_id_from_uuid,
    models::{
        activities::get_activity_by_uuid,
        notes::{Note, NoteType},
        profiles::Profile,
        remote_notes::RemoteNote,
    },
    runner::{
        //encrypted::handle_encrypted_note,
        get_inboxes,
        send_to_inboxes,
        send_to_mq,
    },
    signing::{Method, SignParams},
    MaybeReference,
};

use super::TaskError;
use super::{actor::get_actor, timeline::add_to_timeline};

async fn cache_note(note: &'_ ApNote) -> &'_ ApNote {
    if let Some(attachments) = &note.attachment {
        for attachment in attachments {
            let _ = cache_content(attachment.clone().try_into()).await;
        }
    }

    if let Some(tags) = &note.tag {
        for tag in tags {
            let _ = cache_content(tag.clone().try_into()).await;
        }
    }

    if let Some(metadata_vec) = &note.ephemeral_metadata {
        for metadata in metadata_vec {
            if let Some(og_image) = metadata.og_image.clone() {
                let _ = cache_content(Ok(ApImage::from(og_image).into())).await;
            }

            if let Some(twitter_image) = metadata.twitter_image.clone() {
                let _ = cache_content(Ok(ApImage::from(twitter_image).into())).await;
            }
        }
    }

    note
}

pub async fn delete_note_task(
    conn: Option<Db>,
    channels: Option<EventChannels>,
    uuids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    for uuid in uuids {
        log::debug!("LOOKING FOR UUID {uuid}");

        let (activity, target_note, target_remote_note, target_profile, target_remote_actor) =
            get_activity_by_uuid(conn, uuid.clone())
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
            ),
            None,
        ))
        .map_err(|_| TaskError::TaskFailed)?;
        let inboxes: Vec<ApAddress> = get_inboxes(activity.clone(), sender.clone()).await;

        send_to_inboxes(inboxes, sender, activity.clone()).await;

        let ap_id = get_note_ap_id_from_uuid(note.uuid.clone());

        let records = delete_timeline_item_by_ap_id(conn, ap_id)
            .await
            .map_err(|_| TaskError::TaskFailed)?;
        log::debug!("TIMELINE RECORDS DELETED: {records}");

        let records = delete_note_by_uuid(conn, note.uuid)
            .await
            .map_err(|_| TaskError::TaskFailed)?;
        log::debug!("NOTE RECORDS DELETED: {records}");
    }

    Ok(())
}

pub fn delete_note(job: Job) -> io::Result<()> {
    log::debug!("DELETING NOTE");
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    handle
        .block_on(async {
            delete_note_task(
                None,
                None,
                serde_json::from_value(job.args().into()).unwrap(),
            )
            .await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub async fn outbound_note_task(
    conn: Option<Db>,
    channels: Option<EventChannels>,
    uuids: Vec<String>,
) -> Result<(), TaskError> {
    let conn = conn.as_ref();

    for uuid in uuids {
        let (activity, target_note, target_remote_note, target_profile, target_remote_actor) =
            get_activity_by_uuid(conn, uuid)
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

        let activity = match note.kind {
            NoteType::Note => {
                if let Ok(activity) = ApActivity::try_from((
                    (
                        activity,
                        target_note,
                        target_remote_note,
                        target_profile,
                        target_remote_actor,
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

        add_note_to_timeline(note, sender.clone()).await;

        let activity = activity.ok_or(TaskError::TaskFailed)?;

        let inboxes: Vec<ApAddress> = get_inboxes(activity.clone(), sender.clone()).await;

        log::debug!("SENDING ACTIVITY\n{activity:#?}");
        log::debug!("INBOXES\n{inboxes:#?}");

        for url_str in inboxes {
            let body = Option::from(serde_json::to_string(&activity).unwrap());
            let method = Method::Post;

            let url =
                Url::parse(&url_str.clone().to_string()).map_err(|_| TaskError::TaskFailed)?;

            let signature = crate::signing::sign(SignParams {
                profile: sender.clone(),
                url: url.clone(),
                body: body.clone(),
                method,
            })
            .map_err(|_| TaskError::TaskFailed)?;

            let client = Client::new()
                .post(&url_str.to_string())
                .header("Date", signature.date)
                .header("Digest", signature.digest.unwrap())
                .header("Signature", &signature.signature)
                .header("Content-Type", "application/activity+json")
                .body(body.unwrap());

            let resp = client.send().await.map_err(|_| TaskError::TaskFailed)?;

            match resp.status() {
                StatusCode::ACCEPTED | StatusCode::OK => {
                    log::debug!("SENT TO {url}")
                }
                _ => log::error!("ERROR SENDING TO {url}"),
            }
        }
    }

    Ok(())
}

pub fn process_outbound_note(job: Job) -> io::Result<()> {
    log::debug!("running process_outbound_note job");
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    handle
        .block_on(async {
            outbound_note_task(
                None,
                None,
                serde_json::from_value(job.args().into()).unwrap(),
            )
            .await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub async fn retrieve_context_task(
    conn: Option<Db>,
    channels: Option<EventChannels>,
    ap_ids: Vec<String>,
) -> Result<(), TaskError> {
    //let conn = conn.as_ref();

    let profile = guaranteed_profile(None, None).await;

    for ap_id in ap_ids {
        let profile = profile.clone();

        if let Some(note) = fetch_remote_note(ap_id.to_string(), profile).await {
            log::debug!("REPLIES\n{:#?}", note.replies);

            if let Some(replies) = ApNote::from(note).replies {
                if let Some(MaybeReference::Actual(_first)) = replies.first {}
            }
        }
    }

    Ok(())
}

pub fn retrieve_context(job: Job) -> io::Result<()> {
    log::debug!("running retrieve_context job");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();
    handle
        .block_on(async {
            retrieve_context_task(
                None,
                None,
                serde_json::from_value(job.args().into()).unwrap(),
            )
            .await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub async fn fetch_remote_note(id: String, profile: Profile) -> Option<RemoteNote> {
    log::debug!("PERFORMING REMOTE LOOKUP FOR NOTE: {id}");

    let _url = id.clone();
    let _method = Method::Get;

    if let Ok(resp) = signed_get(profile, id, false).await {
        match resp.status() {
            StatusCode::ACCEPTED | StatusCode::OK => match resp.json().await {
                Ok(ApObject::Note(note)) => {
                    create_or_update_remote_note(None, cache_note(&note).await.clone().into()).await
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

async fn add_note_to_timeline(note: Note, sender: Profile) {
    // Add to local timeline - this checks to be sure that the profile is represented as a remote_actor
    // locally before adding the Note to the Timeline. This is important as the Timeline only uses the
    // remote_actor representation (not the profile) for attributed_to data. In this case, the sender
    // and the note.attributed_to should represent the same person.
    if get_actor(sender, note.clone().attributed_to)
        .await
        .is_some()
    {
        if let Ok(timeline_item) = create_timeline_item(None, note.clone().into()).await {
            add_to_timeline(Option::from(note.clone().ap_to), note.cc, timeline_item).await;
        }
    }
}

pub async fn create_remote_note_tags(remote_note: RemoteNote) {
    let new_tags: Vec<NewRemoteNoteHashtag> = remote_note.clone().into();

    join_all(
        new_tags
            .iter()
            .map(|tag| async { create_remote_note_hashtag(None, tag.clone()).await }),
    )
    .await;
}

pub async fn create_timeline_tags(timeline_item: TimelineItem) {
    let new_tags: Vec<NewTimelineHashtag> = timeline_item.clone().into();

    join_all(
        new_tags
            .iter()
            .map(|tag| async { create_timeline_hashtag(None, tag.clone()).await }),
    )
    .await;
}

pub async fn handle_remote_note(
    channels: Option<EventChannels>,
    remote_note: RemoteNote,
    announcer: Option<String>,
) -> anyhow::Result<RemoteNote> {
    log::debug!("HANDLING REMOTE NOTE");

    let note: ApNote = remote_note.clone().into();
    let profile = guaranteed_profile(None, None);

    let _ = get_actor(profile.await, note.attributed_to.to_string()).await;

    let mut note = cache_note(&note).await.clone();

    if let Some(announcer) = announcer {
        note.ephemeral_announces = Some(vec![announcer]);
    }

    create_remote_note_tags(remote_note.clone()).await;

    if let Ok(timeline_item) = create_timeline_item(None, (None, note.clone()).into()).await {
        create_timeline_tags(timeline_item.clone()).await;

        add_to_timeline(
            remote_note.clone().ap_to,
            remote_note.clone().cc,
            timeline_item,
        )
        .await;

        if let Some(mut channels) = channels {
            channels.send(None, serde_json::to_string(&note.clone()).unwrap());
        }
        //send_to_mq(vec![(String::new(), note.clone())]).await;
    }

    Ok(remote_note)
}

pub fn handle_remote_encrypted_note(remote_note: RemoteNote) -> io::Result<()> {
    log::debug!("adding to processing queue");
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    if let Some(ap_to) = remote_note.clone().ap_to {
        let to_vec: Vec<String> = match serde_json::from_value(ap_to) {
            Ok(x) => x,
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
        };

        to_vec
            .iter()
            .filter_map(|ap_id| {
                handle.block_on(async { get_profile_by_ap_id(None, ap_id.to_string()).await })
            })
            .for_each(|profile| {
                handle.block_on(async {
                    create_processing_item(None, (remote_note.clone(), profile.id).into()).await;
                });
            });
    }

    Ok(())
}

pub fn process_remote_note(job: Job) -> io::Result<()> {
    log::debug!("running process_remote_note job");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    handle
        .block_on(async {
            remote_note_task(
                None,
                None,
                serde_json::from_value(job.args().into()).unwrap(),
            )
            .await
        })
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
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
        match remote_note.kind {
            NoteType::Note => {
                let _ = handle_remote_note(channels, remote_note.clone(), None).await;
            }
            NoteType::EncryptedNote => {
                let _ = handle_remote_encrypted_note(remote_note);
            }
            _ => (),
        }
    }

    Ok(())
}
