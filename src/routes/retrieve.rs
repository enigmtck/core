use std::collections::HashMap;

use anyhow::Result;
use rocket::futures::stream::{self, StreamExt};

use crate::activity_pub::ApTimelineObject;
use crate::fairings::events::EventChannels;
use crate::models::from_serde;
use crate::models::timeline::ContextualizedTimelineItem;
use crate::runner;
use crate::{
    activity_pub::{retriever::get_actor, ApActor, ApCollection, ApObject, ApTag},
    db::Db,
    models::{
        cache::Cache,
        profiles::Profile,
        timeline::{
            get_timeline_items, get_timeline_items_by_conversation, AuthenticatedTimelineItem,
            TimelineFilters, TimelineItem,
        },
    },
};

pub async fn timeline(conn: &Db, limit: i64, offset: i64) -> ApObject {
    process(
        conn,
        get_timeline_items(conn, limit, offset, None, None).await,
        None,
    )
    .await
}

pub async fn inbox(
    conn: &Db,
    limit: i64,
    offset: i64,
    profile: Profile,
    filters: TimelineFilters,
) -> ApObject {
    // the timeline vec will include duplicates due to the table joins

    process(
        conn,
        get_timeline_items(conn, limit, offset, Some(profile.clone()), Some(filters)).await,
        Some(profile),
    )
    .await
}

async fn process(
    conn: &Db,
    timeline: Vec<AuthenticatedTimelineItem>,
    profile: Option<Profile>,
) -> ApObject {
    let objects = process_objects(conn, &timeline, profile).await;

    // Process the notes asynchronously
    let ap_objects: Vec<ApObject> = stream::iter(objects)
        .then(|object| async move {
            match object {
                ApTimelineObject::Note(note) => {
                    ApObject::Note(note.clone().cache(conn).await.clone())
                }
                ApTimelineObject::Question(question) => ApObject::Question(question),
            }
        })
        .collect()
        .await;

    ApObject::Collection(ApCollection::from(ap_objects))
}

async fn process_objects(
    conn: &Db,
    timeline_items: &[AuthenticatedTimelineItem],
    profile: Option<Profile>,
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
            requester: profile.clone(),
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

    let ap_objects: Result<Vec<ApObject>> = conversations
        .iter()
        .map(|item| {
            ContextualizedTimelineItem {
                item: item.clone(),
                ..Default::default()
            }
            .try_into()
            .map(ApObject::Note)
        })
        .collect();

    Ok(ApObject::Collection(ApCollection::from(ap_objects?)))
}
