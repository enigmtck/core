use std::collections::HashMap;

use anyhow::Result;

use crate::activity_pub::{ActivityPub, ApActivity, ApTimelineObject};
use crate::fairings::events::EventChannels;
use crate::models::from_serde;
use crate::models::pg::activities::get_activities_coalesced;
use crate::models::timeline::ContextualizedTimelineItem;
use crate::{
    activity_pub::{retriever::get_actor, ApActor, ApCollection, ApObject, ApTag},
    db::Db,
    models::{
        cache::Cache,
        profiles::Profile,
        timeline::{
            get_timeline_items_by_conversation, get_timeline_items_raw, AuthenticatedTimelineItem,
            TimelineFilters, TimelineItem,
        },
    },
};
use crate::{runner, SERVER_URL};

pub async fn timeline(conn: &Db, limit: i64, min: Option<i64>, max: Option<i64>) -> ApObject {
    let server_url = &*SERVER_URL;
    let base_url = format!("{server_url}/api/timeline?limit={limit}");

    process(
        conn,
        get_timeline_items_raw(conn, limit, min, max, None, None).await,
        None,
        base_url,
    )
    .await
}

pub async fn activities(
    conn: &Db,
    limit: i32,
    min: Option<i64>,
    max: Option<i64>,
    requester: Option<Profile>,
    filters: TimelineFilters,
) -> ApObject {
    let server_url = &*SERVER_URL;
    let base_url = format!("{server_url}/inbox?limit={limit}");

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
    limit: i64,
    min: Option<i64>,
    max: Option<i64>,
    requester: Profile,
    filters: TimelineFilters,
) -> ApObject {
    // the timeline vec will include duplicates due to the table joins
    let server_url = &*SERVER_URL;
    let username = requester.username.clone();
    let base_url = format!("{server_url}/user/{username}/inbox?limit={limit}");

    process(
        conn,
        get_timeline_items_raw(
            conn,
            limit,
            min,
            max,
            Some(requester.clone()),
            Some(filters),
        )
        .await,
        Some(requester),
        base_url,
    )
    .await
}

async fn process(
    conn: &Db,
    timeline: Vec<AuthenticatedTimelineItem>,
    requester: Option<Profile>,
    base_url: String,
) -> ApObject {
    let objects = process_objects(conn, &timeline, requester).await;

    let mut ap_objects: Vec<ActivityPub> = vec![];

    for object in objects.iter() {
        match object {
            ApTimelineObject::Note(note) => {
                ap_objects.push(ActivityPub::Object(ApObject::Note(
                    note.clone().cache(conn).await.clone(),
                )));
            }
            ApTimelineObject::Question(question) => {
                ap_objects.push(ActivityPub::Object(ApObject::Question(question.clone())));
            }
        }
    }

    ApObject::Collection((ap_objects, Some(base_url)).into())
}

async fn process_objects(
    conn: &Db,
    timeline_items: &[AuthenticatedTimelineItem],
    requester: Option<Profile>,
) -> Vec<ApTimelineObject> {
    let mut items: HashMap<String, ContextualizedTimelineItem> = HashMap::new();

    for (item, activity, _activity_to, _activity_cc, _to, cc, _hashtag) in timeline_items {
        let ap_ids = gather_ap_ids(item);
        let ap_actors = get_ap_actors(conn, ap_ids).await;

        let contextualized = ContextualizedTimelineItem {
            item: item.clone(),
            activity: vec![activity.clone()],
            cc: cc.clone().map_or_else(Vec::new, |x| vec![x]),
            related: ap_actors,
            requester: requester.clone(),
        };

        if let Some(captured) = items.get(&item.ap_id) {
            items.insert(item.ap_id.clone(), captured.clone() + contextualized);
        } else {
            items.insert(item.ap_id.clone(), contextualized);
        }
    }

    items
        .values()
        .filter_map(|x| ApTimelineObject::try_from(x.clone()).ok())
        .map(|x| x.dedup())
        .collect()
}

fn gather_ap_ids(x: &TimelineItem) -> Vec<String> {
    let mut ap_ids = vec![x.clone().attributed_to];
    if let Some(tags) = x.clone().tag {
        let tags = from_serde::<Vec<ApTag>>(tags).unwrap_or_default();

        for tag in tags {
            if let ApTag::Mention(tag) = tag {
                if let Some(href) = tag.href {
                    ap_ids.push(href)
                }
            }
        }
    }
    ap_ids
}

async fn get_ap_actors(conn: &Db, ap_ids: Vec<String>) -> Vec<ApActor> {
    let mut ap_actors: Vec<ApActor> = vec![];
    for ap_id in ap_ids {
        if let Some(actor) = get_actor(conn, ap_id.clone(), None, false).await {
            ap_actors.push(actor);
        }
    }
    ap_actors
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
