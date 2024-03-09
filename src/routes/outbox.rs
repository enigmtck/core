use crate::activity_pub::{
    ActivitiesPage, ActivityPub, ApActivity, ApCollectionPage, ApObject, Outbox, Temporal,
};
use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::fairings::signatures::Signed;
use crate::models::activities::{
    get_outbox_activities_by_profile_id, get_outbox_count_by_profile_id,
};
use crate::{activity_pub::ApCollection, models::profiles::get_profile_by_username};
use rocket::{get, http::Status, post, serde::json::Error, serde::json::Json};

use super::ActivityJson;

fn get_published_url(activity: &impl Temporal, username: &str, is_max: bool) -> Option<String> {
    activity.created_at().map(|date| {
        let url = &*crate::SERVER_URL;
        let (min_max, micros) = if is_max {
            ("max", date.timestamp_micros())
        } else {
            ("min", date.timestamp_micros())
        };
        format!("{url}/user/{username}/outbox?page=true&{min_max}={micros}")
    })
}

#[get("/user/<username>/outbox?<min>&<max>&<page>")]
pub async fn outbox_get(
    conn: Db,
    username: String,
    page: Option<bool>,
    min: Option<i64>,
    max: Option<i64>,
) -> Result<ActivityJson<ApObject>, Status> {
    if let Some(profile) = get_profile_by_username((&conn).into(), username.clone()).await {
        if page.is_none() || !page.unwrap() {
            Ok(ActivityJson(Json(ApObject::Collection(
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
            ))))
        } else {
            let activities: Vec<ApActivity> =
                get_outbox_activities_by_profile_id(&conn, profile.id, min, max, Some(5))
                    .await
                    .iter()
                    .filter_map(|x| ApActivity::try_from((x.clone(), None)).ok())
                    .collect::<Vec<ApActivity>>();

            let username = username.clone();

            // this block is particularly challenging to reason around.

            // queries using 'max' are sorted in descending order: the limit number of entries
            // following the created_at max value are returned.

            // queries using 'min' are sorted in ascending order: the limit number of entries
            // following the created_at min value are returned

            // the min/max sets the starting value and the order is important to be sure the
            // correct values are returned.

            // to align with Mastodon, "prev" refers to newer entries and "next" refers to older
            // entries.

            // it would be much simpler to use relative queries (i.e., give me the page*20
            // records and next is page++ with prev as page--) but adopting this more complex
            // approach should make it trivial to integrate caching

            let (prev, next) = match (min, max) {
                // min is specified, so set prev/next accordingly: order is ascending
                // prev (newer) will be a min of the last record
                // next (older) will be a max of the first record
                (Some(_), None) => (
                    activities.last().and_then(|x| match x {
                        ApActivity::Create(y) => get_published_url(y, &username, false),
                        ApActivity::Announce(y) => get_published_url(y, &username, false),
                        _ => None,
                    }),
                    activities.first().and_then(|x| match x {
                        ApActivity::Create(y) => get_published_url(y, &username, true),
                        ApActivity::Announce(y) => get_published_url(y, &username, true),
                        _ => None,
                    }),
                ),

                // max is specified, so set prev/next accordingly: order is descending
                // prev (newer) will be a min of the first record
                // next (older) will be a max of the last record

                // this is also the default when min/max is not specifed (i.e., the first
                // record is the newest in the database)
                (None, Some(_)) | (None, None) => (
                    activities.first().and_then(|x| match x {
                        ApActivity::Create(y) => get_published_url(y, &username, false),
                        ApActivity::Announce(y) => get_published_url(y, &username, false),
                        _ => None,
                    }),
                    activities.last().and_then(|x| match x {
                        ApActivity::Create(y) => get_published_url(y, &username, true),
                        ApActivity::Announce(y) => get_published_url(y, &username, true),
                        _ => None,
                    }),
                ),
                _ => (None, None),
            };

            Ok(ActivityJson(Json(ApObject::CollectionPage(
                ApCollectionPage::from(ActivitiesPage {
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
                }),
            ))))
        }
    } else {
        Err(Status::NoContent)
    }
}

#[post("/user/<username>/outbox", data = "<object>")]
pub async fn outbox_post(
    signed: Signed,
    conn: Db,
    events: EventChannels,
    username: String,
    object: Result<Json<ActivityPub>, Error<'_>>,
) -> Result<String, Status> {
    log::debug!("POSTING TO OUTBOX\n{object:#?}");

    if signed.local() {
        let profile = get_profile_by_username((&conn).into(), username)
            .await
            .ok_or(Status::new(521))?;

        let object = object.map_err(|_| Status::new(522))?;

        match object {
            Json(ActivityPub::Activity(activity)) => activity.outbox(conn, events, profile).await,
            Json(ActivityPub::Object(object)) => object.outbox(conn, events, profile).await,
            _ => Err(Status::new(523)),
        }
    } else {
        Err(Status::Unauthorized)
    }
}
