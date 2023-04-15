use diesel::prelude::*;

use faktory::Job;
use reqwest::{Client, StatusCode};
use std::{collections::HashSet, io};
use tokio::runtime::Runtime;
use webpage::{Webpage, WebpageOptions};

use crate::{
    activity_pub::{ApActivity, ApActor, ApAddress, ApDelete, ApNote, ApObject, Metadata},
    helper::get_note_ap_id_from_uuid,
    models::{notes::Note, profiles::Profile, remote_notes::RemoteNote},
    runner::{
        encrypted::handle_encrypted_note,
        processing::create_processing_item,
        send_to_inboxes, send_to_mq,
        timeline::delete_timeline_item_by_ap_id,
        user::{get_follower_inboxes, get_profile, get_profile_by_ap_id},
    },
    schema::{notes, remote_notes},
    signing::{Method, SignParams},
    MaybeReference,
};

use super::{
    actor::{get_actor, get_remote_actor_by_ap_id},
    timeline::{add_to_timeline, create_timeline_item},
    POOL,
};

pub fn delete_note(job: Job) -> io::Result<()> {
    log::debug!("DELETING NOTE");

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();
        log::debug!("UUID: {uuid}");

        if let Some(note) = get_note_by_uuid(uuid.to_string()) {
            if let Some(profile) = get_profile(note.profile_id) {
                log::debug!("NOTE\n{note:#?}");

                let ap_note = ApNote::from(note);
                log::debug!("AP_NOTE\n{ap_note:#?}");

                match ApDelete::try_from(ap_note) {
                    Ok(delete) => {
                        log::debug!("DELETE\n{delete:#?}");
                        let inboxes = get_follower_inboxes(profile.clone());

                        send_to_inboxes(inboxes, profile, ApObject::Delete(Box::new(delete)));

                        let ap_id = get_note_ap_id_from_uuid(uuid.clone());

                        if let Ok(records) = delete_timeline_item_by_ap_id(ap_id) {
                            log::debug!("TIMELINE RECORDS DELETED: {records}");

                            if let Ok(records) = delete_note_by_uuid(uuid) {
                                log::debug!("NOTE RECORDS DELETED: {records}");
                            }
                        }
                    }
                    Err(e) => log::error!("{e}"),
                }
            }
        }
    }

    Ok(())
}

pub fn process_outbound_note(job: Job) -> io::Result<()> {
    log::debug!("running process_outbound_note job");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();

        if let Some(mut note) = get_note_by_uuid(uuid) {
            // this is the profile where the note was posted to the outbox
            if let Some(sender) = get_profile(note.profile_id) {
                let mut inboxes: HashSet<String> = HashSet::new();

                let create = match note.kind.as_str() {
                    "Note" => {
                        handle_note(&mut note, &mut inboxes, sender.clone()).map(ApActivity::from)
                    }
                    "EncryptedNote" => {
                        handle_encrypted_note(&mut note, &mut inboxes, sender.clone())
                            .map(ApActivity::from)
                    }
                    _ => None,
                };

                if let Some(create) = create {
                    for url in inboxes {
                        let body = Option::from(serde_json::to_string(&create).unwrap());
                        let method = Method::Post;

                        let signature = crate::signing::sign(SignParams {
                            profile: sender.clone(),
                            url: url.clone(),
                            body: body.clone(),
                            method,
                        });

                        let client = Client::new()
                            .post(&url)
                            .header("Date", signature.date)
                            .header("Digest", signature.digest.unwrap())
                            .header("Signature", &signature.signature)
                            .header(
                                "Content-Type",
                                "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"",
                            )
                            .body(body.unwrap());

                        handle.block_on(async {
                            if let Ok(resp) = client.send().await {
                                if let Ok(text) = resp.text().await {
                                    log::debug!("send successful to: {}\n{}", url, text);
                                }
                            }
                        })
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

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();
        handle.block_on(async {
            if let Some(note) = fetch_remote_note(ap_id.to_string()).await {
                log::debug!("REPLIES\n{:#?}", note.replies);

                if let Some(replies) = note.replies {
                    if let Some(MaybeReference::Actual(_first)) = replies.first {}
                }
            }
        });
    }

    Ok(())
}

pub async fn fetch_remote_note(id: String) -> Option<ApNote> {
    log::debug!("PERFORMING REMOTE LOOKUP FOR NOTE: {id}");

    let _url = id.clone();
    let _method = Method::Get;

    let client = Client::new();
    match client
        .get(&id)
        .header(
            "Accept",
            "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"",
        )
        .send()
        .await
    {
        Ok(resp) => match resp.status() {
            StatusCode::ACCEPTED | StatusCode::OK => match resp.json().await {
                Ok(ApObject::Note(note)) => Option::from(note),
                Err(e) => {
                    log::error!("remote note decode error: {e:#?}");
                    Option::None
                }
                _ => Option::None,
            },
            StatusCode::GONE => {
                log::debug!("GONE: {:#?}", resp.status());
                Option::None
            }
            _ => {
                log::debug!("STATUS: {:#?}", resp.status());
                Option::None
            }
        },
        Err(e) => {
            log::debug!("{:#?}", e);
            Option::None
        }
    }
}

pub fn get_note_by_uuid(uuid: String) -> Option<Note> {
    if let Ok(conn) = POOL.get() {
        match notes::table
            .filter(notes::uuid.eq(uuid))
            .first::<Note>(&conn)
            .optional()
        {
            Ok(x) => x,
            Err(_) => Option::None,
        }
    } else {
        Option::None
    }
}

fn handle_note(note: &mut Note, inboxes: &mut HashSet<String>, sender: Profile) -> Option<ApNote> {
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    if let (Ok(mut recipients), Ok(mut cc_recipients)) = (
        serde_json::from_value::<Vec<ApAddress>>(note.clone().ap_to),
        serde_json::from_value::<Vec<ApAddress>>(note.clone().cc.into()),
    ) {
        recipients.append(&mut cc_recipients);

        for recipient in recipients {
            // check if this is the special Public recipient
            if recipient.clone().is_public() {
                // if it is, get all the inboxes for this sender's followers
                inboxes.extend(get_follower_inboxes(sender.clone()));

                // add the special followers address for the sending profile to the
                // note's cc field
                if let Some(cc) = note.clone().cc {
                    let mut cc: Vec<String> = serde_json::from_value(cc).unwrap();
                    if let Some(followers) = ApActor::from(sender.clone()).followers {
                        cc.push(followers)
                    };
                    note.cc = Option::from(serde_json::to_value(cc).unwrap());
                } else {
                    note.cc = Option::from(
                        serde_json::to_value(vec![ApActor::from(sender.clone()).followers])
                            .unwrap(),
                    );
                }

                update_note_cc(note.clone());
            } else if let Some((receiver, _)) = handle
                .block_on(async { get_actor(sender.clone(), recipient.clone().to_string()).await })
            {
                inboxes.insert(receiver.inbox);
            }
        }
    }

    if let Some(_actor) = get_remote_actor_by_ap_id(note.clone().attributed_to) {
        if let Some(timeline_item) = create_timeline_item(ApNote::from(note.clone()).into()) {
            add_to_timeline(
                Option::from(note.clone().ap_to),
                note.clone().cc,
                timeline_item,
            );
        }
    }

    Some(note.clone().into())
}

pub fn get_links(text: String) -> Vec<String> {
    let re = regex::Regex::new(r#"<a href="(.+?)".*?>"#).unwrap();

    re.captures_iter(&text)
        .filter(|cap| {
            !cap[0].to_string().contains("mention")
                && !cap[0].to_string().contains("u-url")
                && !cap[0].contains("hashtag")
                && !cap[1].to_lowercase().contains(".pdf")
        })
        .map(|cap| cap[1].to_string())
        .collect()
}

pub fn process_remote_note(job: Job) -> io::Result<()> {
    log::debug!("running process_remote_note job");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    let ap_ids = job.args();

    match POOL.get() {
        Ok(conn) => {
            for ap_id in ap_ids {
                let ap_id = ap_id.as_str().unwrap().to_string();
                log::debug!("looking for ap_id: {}", ap_id);

                match remote_notes::table
                    .filter(remote_notes::ap_id.eq(ap_id))
                    .first::<RemoteNote>(&conn)
                {
                    Ok(remote_note) => {
                        if remote_note.kind == "Note" {
                            let links = get_links(remote_note.content.clone());
                            log::debug!("{links:#?}");

                            let metadata: Vec<Metadata> = {
                                links
                                    .iter()
                                    .map(|link| Webpage::from_url(link, WebpageOptions::default()))
                                    .filter(|metadata| metadata.is_ok())
                                    .map(|metadata| metadata.unwrap().html.meta.into())
                                    .collect()
                            };

                            let note: ApNote = (remote_note.clone(), Some(metadata)).into();

                            if let Some(timeline_item) = create_timeline_item(note.clone().into()) {
                                add_to_timeline(
                                    remote_note.clone().ap_to,
                                    remote_note.clone().cc,
                                    timeline_item,
                                );

                                handle.block_on(async {
                                    send_to_mq(note.clone()).await;
                                });
                            }
                        } else if remote_note.kind == "EncryptedNote" {
                            // need to resolve ap_to to a profile_id for the command below
                            log::debug!("adding to processing queue");

                            if let Some(ap_to) = remote_note.clone().ap_to {
                                let to_vec: Vec<String> = {
                                    match serde_json::from_value(ap_to) {
                                        Ok(x) => x,
                                        Err(_e) => vec![],
                                    }
                                };

                                for ap_id in to_vec {
                                    if let Some(profile) = get_profile_by_ap_id(ap_id) {
                                        create_processing_item(
                                            (remote_note.clone(), profile.id).into(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => log::error!("error: {:#?}", e),
                }
            }
        }
        Err(e) => log::error!("error: {:#?}", e),
    }

    Ok(())
}

pub fn update_note_cc(note: Note) -> Option<Note> {
    if let Ok(conn) = POOL.get() {
        match diesel::update(notes::table.find(note.id))
            .set(notes::cc.eq(note.cc))
            .get_result::<Note>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

#[derive(Debug)]
pub enum DeleteNoteError {
    ConnectionError,
    DatabaseError(diesel::result::Error),
}

pub fn delete_note_by_uuid(uuid: String) -> Result<usize, DeleteNoteError> {
    if let Ok(conn) = POOL.get() {
        match diesel::delete(notes::table.filter(notes::uuid.eq(uuid))).execute(&conn) {
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
