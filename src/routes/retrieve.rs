use std::collections::{hash_map::Entry, HashMap};

use anyhow::Result;
use rocket::futures::future::join_all;
use rocket::futures::stream::{self, StreamExt};

use crate::activity_pub::ApTimelineObject;
use crate::fairings::events::EventChannels;
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
    let consolidated_objects = consolidate_objects(objects);

    // Process the notes asynchronously
    let ap_objects: Vec<ApObject> = stream::iter(consolidated_objects)
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
    join_all(timeline_items.iter().map(
        |(timeline_item, activity, _activity_to, _activity_cc, _to, cc, _hashtag)| {
            let profile = profile.clone();
            async move {
                let ap_ids = gather_ap_ids(timeline_item);
                let ap_actors = get_ap_actors(conn, ap_ids).await;

                ContextualizedTimelineItem {
                    item: timeline_item.clone(),
                    activity: Some(activity.clone()),
                    cc: cc.clone(),
                    related: Some(ap_actors),
                    requester: profile,
                }
                .try_into()
                .ok()
            }
        },
    ))
    .await
    .into_iter()
    .filter_map(|x| x) // Use filter_map here to filter out None values
    .collect::<Vec<ApTimelineObject>>() // Collect the results into Vec<ApTimelineObject>
}

fn gather_ap_ids(x: &TimelineItem) -> Vec<String> {
    let mut ap_ids = vec![x.clone().attributed_to];
    if let Some(tags) = x.clone().tag {
        cfg_if::cfg_if! {
            if #[cfg(feature = "pg")] {
                let tags = serde_json::from_value::<Vec<ApTag>>(tags).unwrap_or_default();
            } else if #[cfg(feature = "sqlite")] {
                let tags = serde_json::from_str::<Vec<ApTag>>(&tags).unwrap_or_default();
            }
        }

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

// This function takes the output from a joined database query that includes duplicate
// entries as a result of the join, and merges them together in to a more streamlined
// Vec
fn consolidate_objects(objects: Vec<ApTimelineObject>) -> Vec<ApTimelineObject> {
    let mut consolidated_objects: HashMap<String, ApTimelineObject> = HashMap::new();
    for object in objects {
        match object {
            ApTimelineObject::Note(note) => {
                if let Some(id) = note.id.clone() {
                    match consolidated_objects.entry(id) {
                        Entry::Occupied(mut entry) => {
                            if let ApTimelineObject::Note(consolidated) = entry.get_mut() {
                                // Each entry may contain a single-member vec with an announce
                                // This block checks to see if the entry already exists, and then
                                // merges the announce in to the existing vec
                                if let (Some(existing), Some(announces)) = (
                                    consolidated.ephemeral_announces.as_mut(),
                                    note.ephemeral_announces,
                                ) {
                                    existing.extend(announces);
                                }

                                // Each entry may contain a single-member vec with a like
                                // This block checks to see if the entry already exists, and then
                                // merges the announce in to the existing vec
                                if let (Some(existing), Some(likes)) =
                                    (consolidated.ephemeral_likes.as_mut(), note.ephemeral_likes)
                                {
                                    existing.extend(likes);
                                }

                                // This block looks for a record with a Some liked UUID and
                                // updates the consolidated record with it if one exists
                                // The query excludes any records that are revoked, so the presence
                                // of a single Some is indication that this should be updated
                                if let Some(liked) = note.ephemeral_liked {
                                    consolidated.ephemeral_liked = Some(liked);
                                }

                                // This block looks for a record with a Some announced UUID and
                                // updates the consolidated record with it if one exists
                                // The query excludes any records that are revoked, so the presence
                                // of a single Some is indication that this should be updated
                                if let Some(announced) = note.ephemeral_announced {
                                    consolidated.ephemeral_announced = Some(announced);
                                }
                            }
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(ApTimelineObject::Note(note));
                        }
                    }
                }
            }
            ApTimelineObject::Question(question) => {
                match consolidated_objects.entry(question.id.clone()) {
                    Entry::Occupied(mut entry) => {
                        if let ApTimelineObject::Question(consolidated) = entry.get_mut() {}
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(ApTimelineObject::Question(question));
                    }
                }
            }
        }
    }
    consolidated_objects.values().cloned().collect()
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
