use crate::db::runner::DbRunner;
use crate::models::activities::get_activities_coalesced;
use crate::models::activities::{get_outbox_count_by_actor_id, TimelineFilters};
use crate::models::actors::Actor;
use jdt_activity_pub::{ActivityPub, ApActivity, ApCollection, ApCollectionParams, ApObject};
pub async fn outbox_collection<C: DbRunner>(
    conn: &C,
    profile: Actor,
    //base_url: Option<String>,
    limit: u8,
) -> ApObject {
    let server_url = format!("https://{}", *crate::SERVER_NAME);
    let username = profile.ek_username.unwrap();
    //let base_url = base_url.unwrap_or(format!("{server_url}/{username}/outbox"));
    let base_url = format!("{server_url}/user/{username}/outbox");
    let total_items = get_outbox_count_by_actor_id(conn, profile.id)
        .await
        .unwrap_or(0);

    ApObject::Collection(ApCollection::from(ApCollectionParams {
        total_items,
        base_url,
        limit,
    }))
}

pub async fn activities(
    conn: &impl DbRunner,
    limit: i32,
    min: Option<i64>,
    max: Option<i64>,
    requester: Option<Actor>,
    filters: TimelineFilters,
    base_url: String,
) -> ApObject {
    //let server_url = format!("https://{}", *crate::SERVER_NAME);
    //let base_url = base_url.unwrap_or(format!("{server_url}/inbox?page=true&limit={limit}"));

    let activities = match get_activities_coalesced(
        conn,
        limit,
        min,
        max,
        requester.clone(),
        Some(filters.clone()),
        None,
        None,
        None,
    )
    .await
    {
        Ok(activities) => activities,
        Err(e) => {
            log::error!("{e}");
            vec![]
        }
    };

    let activities = activities
        .into_iter()
        .filter_map(|activity| ApActivity::try_from(activity.clone()).ok())
        .map(ActivityPub::from)
        .collect();

    ApObject::Collection(ApCollection::from((activities, Some(base_url))))
}
