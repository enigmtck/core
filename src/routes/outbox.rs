use crate::activity_pub::{
    ActivitiesPage, ActivityPub, ApActivity, ApCollectionPage, ApNote, ApNoteType, ApObject,
};
use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::fairings::faktory::FaktoryConnection;
use crate::fairings::signatures::Signed;
use crate::models::activities::{
    get_outbox_activities_by_profile_id, get_outbox_count_by_profile_id,
};
use crate::models::notes::get_notes_by_profile_id;
use crate::outbox;
use crate::signing::VerificationType;
use crate::{activity_pub::ApCollection, models::profiles::get_profile_by_username};
use chrono::DateTime;
use rocket::{get, http::Status, post, serde::json::Error, serde::json::Json};

#[get("/user/<username>/noutbox?<min>&<max>&<page>")]
pub async fn outbox_getn(
    conn: Db,
    username: String,
    page: Option<bool>,
    min: Option<i64>,
    max: Option<i64>,
) -> Result<Json<ApObject>, Status> {
    if let Some(profile) = get_profile_by_username(&conn, username.clone()).await {
        if page.is_none() || !page.unwrap() {
            Ok(Json(ApObject::Collection(
                ApCollection::default()
                    .total_items(
                        get_outbox_count_by_profile_id(&conn, profile.id)
                            .await
                            .map(|x| x as u32),
                    )
                    .first(format!(
                        "{}/user/{}/outbox?page=true",
                        *crate::SERVER_URL,
                        username
                    ))
                    .last(format!(
                        "{}/user/{}/outbox?min=0&page=true",
                        *crate::SERVER_URL,
                        username
                    ))
                    .id(format!("{}/user/{}/outbox", *crate::SERVER_URL, username))
                    .ordered()
                    .clone(),
            )))
        } else {
            let activities: Vec<ApActivity> =
                get_outbox_activities_by_profile_id(&conn, profile.id, min, max, Some(2))
                    .await
                    .iter()
                    .filter_map(|x| ApActivity::try_from((x.clone(), None)).ok())
                    .collect::<Vec<ApActivity>>();

            let username = username.clone();
            let (prev, next) = match (min, max) {
                (Some(_min), None) => (
                    activities.first().and_then(|x| match x {
                        ApActivity::Create(y) => y.published.clone().and_then(|z| {
                            DateTime::parse_from_rfc3339(&z).ok().map(|a| {
                                format!(
                                    "{}/user/{}/outbox?page=true&max={}",
                                    *crate::SERVER_URL,
                                    username,
                                    a.timestamp_millis()
                                )
                            })
                        }),
                        ApActivity::Announce(y) => y.published.clone().and_then(|z| {
                            DateTime::parse_from_rfc3339(&z).ok().map(|a| {
                                format!(
                                    "{}/user/{}/outbox?page=true&max={}",
                                    *crate::SERVER_URL,
                                    username,
                                    a.timestamp_millis()
                                )
                            })
                        }),
                        _ => None,
                    }),
                    activities.last().and_then(|x| match x {
                        ApActivity::Create(y) => y.published.clone().and_then(|z| {
                            DateTime::parse_from_rfc3339(&z).ok().map(|a| {
                                format!(
                                    "{}/user/{}/outbox?page=true&min={}",
                                    *crate::SERVER_URL,
                                    username,
                                    a.timestamp_millis()
                                )
                            })
                        }),
                        ApActivity::Announce(y) => y.published.clone().and_then(|z| {
                            DateTime::parse_from_rfc3339(&z).ok().map(|a| {
                                format!(
                                    "{}/user/{}/outbox?page=true&min={}",
                                    *crate::SERVER_URL,
                                    username,
                                    a.timestamp_millis()
                                )
                            })
                        }),
                        _ => None,
                    }),
                ),
                (None, Some(_)) | (None, None) => (
                    activities.last().and_then(|x| match x {
                        ApActivity::Create(y) => y.published.clone().and_then(|z| {
                            DateTime::parse_from_rfc3339(&z).ok().map(|a| {
                                format!(
                                    "{}/user/{}/outbox?page=true&max={}",
                                    *crate::SERVER_URL,
                                    username,
                                    a.timestamp_millis()
                                )
                            })
                        }),
                        ApActivity::Announce(y) => y.published.clone().and_then(|z| {
                            DateTime::parse_from_rfc3339(&z).ok().map(|a| {
                                format!(
                                    "{}/user/{}/outbox?page=true&max={}",
                                    *crate::SERVER_URL,
                                    username,
                                    a.timestamp_millis()
                                )
                            })
                        }),
                        _ => None,
                    }),
                    activities.first().and_then(|x| match x {
                        ApActivity::Create(y) => y.published.clone().and_then(|z| {
                            DateTime::parse_from_rfc3339(&z).ok().map(|a| {
                                format!(
                                    "{}/user/{}/outbox?page=true&min={}",
                                    *crate::SERVER_URL,
                                    username,
                                    a.timestamp_millis()
                                )
                            })
                        }),
                        ApActivity::Announce(y) => y.published.clone().and_then(|z| {
                            DateTime::parse_from_rfc3339(&z).ok().map(|a| {
                                format!(
                                    "{}/user/{}/outbox?page=true&min={}",
                                    *crate::SERVER_URL,
                                    username,
                                    a.timestamp_millis()
                                )
                            })
                        }),
                        _ => None,
                    }),
                ),
                _ => (None, None),
            };

            Ok(Json(ApObject::CollectionPage(ApCollectionPage::from(
                ActivitiesPage {
                    profile,
                    activities,
                    first: Some(format!(
                        "{}/user/{}/outbox?page=true",
                        *crate::SERVER_URL,
                        username
                    )),
                    last: Some(format!(
                        "{}/user/{}/outbox?page=true&min=0",
                        *crate::SERVER_URL,
                        username
                    )),
                    prev,
                    next,
                    part_of: Some(format!("{}/user/{}/outbox", *crate::SERVER_URL, username)),
                },
            ))))
        }
    } else {
        Err(Status::NoContent)
    }
}

#[get("/user/<username>/outbox?<offset>&<limit>")]
pub async fn outbox_get(
    conn: Db,
    username: String,
    offset: u16,
    limit: u8,
) -> Result<Json<ApCollection>, Status> {
    if let Some(profile) = get_profile_by_username(&conn, username).await {
        let notes: Vec<ApObject> =
            get_notes_by_profile_id(&conn, profile.id, limit.into(), offset.into(), true)
                .await
                .iter()
                .map(|note| ApObject::Note(ApNote::from(note.clone())))
                .collect();
        Ok(Json(ApCollection::from(notes)))
    } else {
        Err(Status::NoContent)
    }
}

#[post("/user/<username>/outbox", data = "<object>")]
pub async fn outbox_post(
    signed: Signed,
    conn: Db,
    faktory: FaktoryConnection,
    events: EventChannels,
    username: String,
    object: Result<Json<ActivityPub>, Error<'_>>,
) -> Result<String, Status> {
    log::debug!("POSTING TO OUTBOX\n{object:#?}");

    if let Signed(true, VerificationType::Local) = signed {
        match get_profile_by_username(&conn, username).await {
            Some(profile) => match object {
                Ok(object) => match object {
                    Json(ActivityPub::Activity(activity)) => match activity {
                        ApActivity::Undo(activity) => {
                            outbox::activity::undo(conn, faktory, *activity, profile).await
                        }
                        ApActivity::Follow(activity) => {
                            outbox::activity::follow(conn, faktory, activity, profile).await
                        }
                        ApActivity::Like(activity) => {
                            outbox::activity::like(conn, faktory, *activity, profile).await
                        }
                        ApActivity::Announce(activity) => {
                            outbox::activity::announce(conn, faktory, activity, profile).await
                        }
                        ApActivity::Delete(activity) => {
                            outbox::activity::delete(conn, faktory, *activity, profile).await
                        }
                        _ => Err(Status::NoContent),
                    },
                    Json(ActivityPub::Object(ApObject::Note(note))) => {
                        // EncryptedNotes need to be handled differently, but use the ApNote struct
                        match note.kind {
                            ApNoteType::Note => {
                                outbox::object::note(conn, faktory, events, note, profile).await
                            }
                            ApNoteType::EncryptedNote => {
                                outbox::object::encrypted_note(conn, faktory, events, note, profile)
                                    .await
                            }
                            _ => Err(Status::NoContent),
                        }
                    }
                    Json(ActivityPub::Object(ApObject::Session(session))) => {
                        outbox::object::session(conn, faktory, session, profile).await
                    }
                    _ => Err(Status::NoContent),
                },
                Err(_) => Err(Status::NoContent),
            },
            None => Err(Status::NoContent),
        }
    } else {
        Err(Status::NoContent)
    }
}
