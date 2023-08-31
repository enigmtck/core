use std::collections::{hash_map::Entry, HashMap};

use rocket::futures::future::join_all;
use rocket::futures::stream::{self, StreamExt};

use crate::{
    activity_pub::{
        retriever::get_actor, ApActor, ApCollection, ApNote, ApObject, ApTag,
        FullyQualifiedTimelineItem,
    },
    db::Db,
    fairings::faktory::{assign_to_faktory, FaktoryConnection},
    models::{
        cache::Cache,
        profiles::Profile,
        timeline::{
            get_authenticated_timeline_items, get_public_timeline_items,
            get_timeline_items_by_conversation, AuthenticatedTimelineItem, TimelineItem,
        },
    },
};

pub async fn timeline(conn: &Db, limit: i64, offset: i64) -> ApObject {
    let timeline = get_public_timeline_items(conn, limit, offset).await;

    let items = timeline
        .iter()
        .map(|(timeline, activity)| (timeline.clone(), activity.clone(), None))
        .collect();

    process(conn, items, None).await
}

pub async fn inbox(conn: &Db, limit: i64, offset: i64, profile: Profile) -> ApObject {
    // the timeline vec will include duplicates due to the table joins
    process(
        conn,
        get_authenticated_timeline_items(conn, limit, offset, profile.clone()).await,
        Some(profile),
    )
    .await
}

async fn process(
    conn: &Db,
    timeline: Vec<AuthenticatedTimelineItem>,
    profile: Option<Profile>,
) -> ApObject {
    let notes = process_notes(conn, &timeline, profile).await;
    let consolidated_notes = consolidate_notes(notes);

    // Process the notes asynchronously
    let ap_objects: Vec<ApObject> = stream::iter(consolidated_notes)
        .then(|note| async move { ApObject::Note(note.clone().cache(conn).await.clone()) })
        .collect()
        .await;

    ApObject::Collection(ApCollection::from(ap_objects))
}

async fn process_notes(
    conn: &Db,
    timeline_items: &[AuthenticatedTimelineItem],
    profile: Option<Profile>,
) -> Vec<ApNote> {
    join_all(timeline_items.iter().map(|(timeline_item, activity, cc)| {
        let profile = profile.clone();
        async move {
            let ap_ids = gather_ap_ids(timeline_item);
            let ap_actors = get_ap_actors(conn, ap_ids).await;
            let fully_qualified_timeline_item: FullyQualifiedTimelineItem = (
                (timeline_item.clone(), activity.clone(), cc.clone()),
                Some(ap_actors),
                profile,
            );
            fully_qualified_timeline_item.into()
        }
    }))
    .await
}

fn gather_ap_ids(x: &TimelineItem) -> Vec<String> {
    let mut ap_ids = vec![x.clone().attributed_to];
    if let Some(tags) = x.clone().tag {
        if let Ok(tags) = serde_json::from_value::<Vec<ApTag>>(tags) {
            for tag in tags {
                if let ApTag::Mention(tag) = tag {
                    if let Some(href) = tag.href {
                        ap_ids.push(href)
                    }
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
fn consolidate_notes(notes: Vec<ApNote>) -> Vec<ApNote> {
    let mut consolidated_notes: HashMap<String, ApNote> = HashMap::new();
    for note in notes {
        if let Some(id) = note.id.clone() {
            match consolidated_notes.entry(id) {
                Entry::Occupied(mut entry) => {
                    let consolidated = entry.get_mut();

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
                Entry::Vacant(entry) => {
                    entry.insert(note);
                }
            }
        }
    }
    consolidated_notes.values().cloned().collect()
}

pub async fn conversation(
    conn: &Db,
    faktory: FaktoryConnection,
    conversation: String,
    limit: i64,
    offset: i64,
) -> ApObject {
    let conversation = get_timeline_items_by_conversation(conn, conversation, limit, offset).await;

    match assign_to_faktory(
        faktory,
        String::from("retrieve_context"),
        vec![conversation[0].clone().ap_id],
    ) {
        Ok(_) => log::debug!("ASSIGNED TO FAKTORY"),
        Err(e) => log::error!("FAILED TO ASSIGN TO FAKTORY\n{e:#?}"),
    }

    ApObject::Collection(ApCollection::from(
        conversation
            .iter()
            .map(|x| ApObject::Note(ApNote::from((x.clone(), None))))
            .collect::<Vec<ApObject>>(),
    ))
}
