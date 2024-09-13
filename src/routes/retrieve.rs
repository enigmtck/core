use anyhow::Result;

use crate::activity_pub::{ActivityPub, ApActivity};
use crate::fairings::events::EventChannels;
use crate::models::pg::activities::get_activities_coalesced;
use crate::models::timeline::ContextualizedTimelineItem;
use crate::{
    activity_pub::{ApCollection, ApObject},
    db::Db,
    models::{
        profiles::Profile,
        timeline::{get_timeline_items_by_conversation, TimelineFilters},
    },
};
use crate::{runner, SERVER_URL};

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
    let base_url = base_url.unwrap_or(format!("{server_url}/inbox?limit={limit}"));

    let activities = get_activities_coalesced(
        conn,
        limit,
        min,
        max,
        requester.clone(),
        Some(filters.clone()),
    )
    .await;

    log::debug!("ACTIVITIES:{activities:#?}");

    let activities = activities
        .iter()
        .filter_map(|activity| ApActivity::try_from(activity.clone()).ok())
        .map(ActivityPub::from)
        .collect();

    ApObject::Collection(ApCollection::from((activities, Some(base_url))))
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
    let base_url = format!("{server_url}/user/{username}/inbox?limit={limit}");

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

pub async fn conversation(
    conn: Db,
    channels: EventChannels,
    conversation: String,
    limit: i64,
    offset: i64,
) -> Result<ApObject> {
    let conversations =
        get_timeline_items_by_conversation(Some(&conn), conversation.clone(), limit, offset)
            .await
            .unwrap_or(vec![]);

    if let Some(top) = conversations.first() {
        runner::run(
            runner::note::retrieve_context_task,
            Some(conn),
            Some(channels),
            vec![top.ap_id.clone()],
        )
        .await;
    }

    let ap_objects: Result<Vec<ActivityPub>> = conversations
        .iter()
        .map(|item| {
            ContextualizedTimelineItem {
                item: item.clone(),
                ..Default::default()
            }
            .try_into()
            .map(ApObject::Note)
            .map(ActivityPub::Object)
        })
        .collect();

    Ok(ApObject::Collection(ApCollection::from((
        ap_objects?,
        None,
    ))))
}
