use std::io::ErrorKind;

use diesel::prelude::*;

use faktory::Job;
use reqwest::{Client, StatusCode};
use tokio::io;
use tokio::runtime::Runtime;
use url::Url;

use crate::activity_pub::retriever::signed_get;
use crate::activity_pub::ApImage;
use crate::models::profiles::guaranteed_profile;
use crate::models::timeline::{create_timeline_item, delete_timeline_item_by_ap_id};
use crate::runner::cache::cache_content;
use crate::{
    activity_pub::{ApActivity, ApAddress, ApNote, ApObject},
    helper::get_note_ap_id_from_uuid,
    models::{
        activities::get_activity_by_uuid,
        notes::{Note, NoteType},
        profiles::Profile,
        remote_notes::{NewRemoteNote, RemoteNote},
    },
    runner::{
        //encrypted::handle_encrypted_note,
        get_inboxes,
        processing::create_processing_item,
        send_to_inboxes,
        send_to_mq,
        user::{get_profile, get_profile_by_ap_id},
    },
    schema::{notes, remote_notes},
    signing::{Method, SignParams},
    MaybeReference, POOL,
};

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

pub fn create_or_update_remote_note(note: NewRemoteNote) -> Option<RemoteNote> {
    if let Ok(mut conn) = POOL.get() {
        diesel::insert_into(remote_notes::table)
            .values(&note)
            .on_conflict(remote_notes::ap_id)
            .do_update()
            .set(&note)
            .get_result::<RemoteNote>(&mut conn)
            .ok()
    } else {
        None
    }
}

pub fn delete_note(job: Job) -> io::Result<()> {
    log::debug!("DELETING NOTE");
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();
        log::debug!("LOOKING FOR UUID {uuid}");

        if let Some((
            activity,
            target_note,
            target_remote_note,
            target_profile,
            target_remote_actor,
        )) = handle.block_on(async {
            get_activity_by_uuid(POOL.get().expect("failed to get pool").into(), uuid.clone()).await
        }) {
            log::debug!("FOUND ACTIVITY\n{activity:#?}");
            if let Some(profile_id) = activity.profile_id {
                if let (Some(sender), Some(note)) = (get_profile(profile_id), target_note.clone()) {
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
                        let inboxes: Vec<ApAddress> = handle.block_on(async {
                            get_inboxes(activity.clone(), sender.clone()).await
                        });
                        handle.block_on(async {
                            send_to_inboxes(inboxes, sender, activity.clone()).await
                        });

                        let ap_id = get_note_ap_id_from_uuid(note.uuid.clone());

                        if let Ok(records) = handle.block_on(async move {
                            delete_timeline_item_by_ap_id(
                                POOL.get()
                                    .expect("failed to get database connection")
                                    .into(),
                                ap_id,
                            )
                            .await
                        }) {
                            log::debug!("TIMELINE RECORDS DELETED: {records}");

                            if let Ok(records) = delete_note_by_uuid(note.uuid) {
                                log::debug!("NOTE RECORDS DELETED: {records}");
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn process_outbound_note(job: Job) -> io::Result<()> {
    log::debug!("running process_outbound_note job");
    let runtime = Runtime::new().unwrap();
    let handle = runtime.handle();

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();

        if let Some((
            activity,
            target_note,
            target_remote_note,
            target_profile,
            target_remote_actor,
        )) = handle.block_on(async {
            get_activity_by_uuid(POOL.get().expect("failed to get pool").into(), uuid).await
        }) {
            if let Some(profile_id) = activity.profile_id {
                if let (Some(sender), Some(note)) = (get_profile(profile_id), target_note.clone()) {
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

                    handle.block_on(async { add_note_to_timeline(note, sender.clone()).await });

                    if let Some(activity) = activity {
                        let inboxes: Vec<ApAddress> = handle.block_on(async {
                            get_inboxes(activity.clone(), sender.clone()).await
                        });

                        log::debug!("SENDING ACTIVITY\n{activity:#?}");
                        log::debug!("INBOXES\n{inboxes:#?}");

                        for url_str in inboxes {
                            let body = Option::from(serde_json::to_string(&activity).unwrap());
                            let method = Method::Post;

                            if let Ok(url) = Url::parse(&url_str.clone().to_string()) {
                                if let Ok(signature) = crate::signing::sign(SignParams {
                                    profile: sender.clone(),
                                    url: url.clone(),
                                    body: body.clone(),
                                    method,
                                }) {
                                    let client = Client::new()
                                        .post(&url_str.to_string())
                                        .header("Date", signature.date)
                                        .header("Digest", signature.digest.unwrap())
                                        .header("Signature", &signature.signature)
                                        .header("Content-Type", "application/activity+json")
                                        .body(body.unwrap());

                                    if let Ok(resp) = handle.block_on(async { client.send().await })
                                    {
                                        match resp.status() {
                                            StatusCode::ACCEPTED | StatusCode::OK => {
                                                log::debug!("SENT TO {url}")
                                            }
                                            _ => log::error!("ERROR SENDING TO {url}"),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn retrieve_context(job: Job) -> io::Result<()> {
    log::debug!("running retrieve_context job");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();
    let pool = POOL.get().map_err(|_| io::Error::from(ErrorKind::Other))?;
    let profile = handle.block_on(async { guaranteed_profile(pool.into(), None).await });

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();
        let profile = profile.clone();

        if let Some(note) =
            handle.block_on(async { fetch_remote_note(ap_id.to_string(), profile).await })
        {
            log::debug!("REPLIES\n{:#?}", note.replies);

            if let Some(replies) = ApNote::from(note).replies {
                if let Some(MaybeReference::Actual(_first)) = replies.first {}
            }
        }
    }

    Ok(())
}

pub async fn fetch_remote_note(id: String, profile: Profile) -> Option<RemoteNote> {
    log::debug!("PERFORMING REMOTE LOOKUP FOR NOTE: {id}");

    let _url = id.clone();
    let _method = Method::Get;

    if let Ok(resp) = signed_get(profile, id, false).await {
        match resp.status() {
            StatusCode::ACCEPTED | StatusCode::OK => match resp.json().await {
                Ok(ApObject::Note(note)) => {
                    create_or_update_remote_note(cache_note(&note).await.clone().into())
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

pub fn get_note_by_uuid(uuid: String) -> Option<Note> {
    if let Ok(mut conn) = POOL.get() {
        match notes::table
            .filter(notes::uuid.eq(uuid))
            .first::<Note>(&mut conn)
            .optional()
        {
            Ok(x) => x,
            Err(_) => Option::None,
        }
    } else {
        Option::None
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

pub async fn handle_remote_note(remote_note: RemoteNote) -> anyhow::Result<RemoteNote> {
    log::debug!("HANDLING REMOTE NOTE");

    let note: ApNote = remote_note.clone().into();
    let pool = POOL.get()?;
    let profile = guaranteed_profile(pool.into(), None);

    let _ = get_actor(profile.await, note.attributed_to.to_string()).await;

    let note = cache_note(&note).await.clone();

    if let Ok(timeline_item) = create_timeline_item(None, (None, note.clone()).into()).await {
        add_to_timeline(
            remote_note.clone().ap_to,
            remote_note.clone().cc,
            timeline_item,
        )
        .await;

        send_to_mq(note.clone()).await;
    }

    Ok(remote_note)
}

fn handle_remote_encrypted_note(remote_note: RemoteNote) -> io::Result<()> {
    log::debug!("adding to processing queue");

    if let Some(ap_to) = remote_note.clone().ap_to {
        let to_vec: Vec<String> = match serde_json::from_value(ap_to) {
            Ok(x) => x,
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
        };

        to_vec
            .iter()
            .filter_map(|ap_id| get_profile_by_ap_id(ap_id.to_string()))
            .for_each(|profile| {
                create_processing_item((remote_note.clone(), profile.id).into());
            });
    }

    Ok(())
}

pub fn process_remote_note(job: Job) -> io::Result<()> {
    log::debug!("running process_remote_note job");

    let ap_ids = job.args();
    if let Ok(mut conn) = POOL.get() {
        let rt = Runtime::new().unwrap();
        let handle = rt.handle();

        for ap_id in ap_ids {
            let ap_id = ap_id.as_str().unwrap().to_string();
            log::debug!("looking for ap_id: {}", ap_id);

            match remote_notes::table
                .filter(remote_notes::ap_id.eq(ap_id))
                .first::<RemoteNote>(&mut conn)
            {
                Ok(remote_note) => {
                    if remote_note.kind == "Note" {
                        handle.block_on(async {
                            let _ = handle_remote_note(remote_note.clone()).await;
                        });
                    } else if remote_note.kind == "EncryptedNote" {
                        handle_remote_encrypted_note(remote_note)?;
                    }
                }
                Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
pub enum DeleteNoteError {
    ConnectionError,
    DatabaseError(diesel::result::Error),
}

pub fn delete_note_by_uuid(uuid: String) -> Result<usize, DeleteNoteError> {
    if let Ok(mut conn) = POOL.get() {
        match diesel::delete(notes::table.filter(notes::uuid.eq(uuid))).execute(&mut conn) {
            Ok(x) => Ok(x),
            Err(e) => {
                log::error!("FAILED TO DELETE\n{e:#?}");
                Err(DeleteNoteError::DatabaseError(e))
            }
        }
    } else {
        Err(DeleteNoteError::ConnectionError)
    }
}
