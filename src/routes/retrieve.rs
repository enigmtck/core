use crate::models::activities::{get_outbox_count_by_profile_id, TimelineFilters};
use crate::models::pg::activities::get_activities_coalesced;
use crate::SERVER_URL;
use crate::{
    activity_pub::{ActivityPub, ApActivity, ApCollection, ApCollectionPage, ApObject},
    db::Db,
    models::profiles::Profile,
};

pub async fn outbox_collection(conn: &Db, profile: Profile, base_url: Option<String>) -> ApObject {
    let server_url = &*SERVER_URL;
    let username = profile.username;
    let base_url = base_url.unwrap_or(format!("{server_url}/{username}/outbox"));
    let count = get_outbox_count_by_profile_id(conn, profile.id)
        .await
        .unwrap_or(0);

    ApObject::Collection(ApCollection::from((count, base_url)))
}

pub async fn activities(
    conn: &Db,
    limit: i32,
    min: Option<i64>,
    max: Option<i64>,
    requester: Option<Profile>,
    filters: TimelineFilters,
    base_url: Option<String>,
) -> ApObject {
    let server_url = &*SERVER_URL;
    let base_url = base_url.unwrap_or(format!("{server_url}/inbox?page=true&limit={limit}"));

    let activities = get_activities_coalesced(
        conn,
        limit,
        min,
        max,
        requester.clone(),
        Some(filters.clone()),
        None,
    )
    .await;

    let activities = activities
        .into_iter()
        .filter_map(|activity| ApActivity::try_from(activity.clone()).ok())
        .map(ActivityPub::from)
        .collect();

    ApObject::CollectionPage(ApCollectionPage::from((activities, Some(base_url))))
}

pub async fn inbox(
    conn: &Db,
    limit: i32,
    min: Option<i64>,
    max: Option<i64>,
    requester: Profile,
    filters: TimelineFilters,
) -> ApObject {
    let server_url = &*SERVER_URL;
    let username = requester.username.clone();
    let base_url = format!("{server_url}/user/{username}/inbox?page=true&limit={limit}");

    activities(
        conn,
        limit,
        min,
        max,
        Some(requester),
        filters,
        Some(base_url),
    )
    .await
}
