use std::collections::{hash_map::Entry, HashMap, HashSet};

use rocket::futures::future::join_all;

use crate::{
    activity_pub::{
        retriever::get_actor, ApActor, ApCollection, ApNote, ApObject, ApTag,
        FullyQualifiedTimelineItem,
    },
    db::Db,
    fairings::faktory::{assign_to_faktory, FaktoryConnection},
    models::{
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
        .map(|(timeline, announce, like)| {
            (
                timeline.clone(),
                None,
                None,
                None,
                announce.clone(),
                like.clone(),
            )
        })
        .collect();

    process(conn, items).await
}

pub async fn inbox(conn: &Db, limit: i64, offset: i64, profile: Profile) -> ApObject {
    // the timeline vec will include duplicates due to the table joins
    process(
        conn,
        get_authenticated_timeline_items(conn, limit, offset, profile).await,
    )
    .await
}

// // TODO: this is too complex
// async fn process(conn: &Db, timeline: Vec<AuthenticatedTimelineItem>) -> ApObject {
//     // the async here is due to the get_actor call below
//     let notes: Vec<ApNote> = join_all(timeline.iter().map(|(x, l, a, c, ra, rl)| async {
//         let mut ap_ids = vec![x.clone().attributed_to];
//         let mut ap_actors: Vec<ApActor> = vec![];

//         if let Some(tags) = x.clone().tag {
//             if let Ok(tags) = serde_json::from_value::<Vec<ApTag>>(tags) {
//                 for tag in tags {
//                     if let ApTag::Mention(tag) = tag {
//                         if let Some(href) = tag.href {
//                             ap_ids.push(href)
//                         }
//                     }
//                 }
//             }
//         }

//         // we populate the actors with data that exists in the database to reduce external
//         // calls by the client; we specify "false" on the option to get_actor to tell it not
//         // to have the server retrieve the data externally, however (those external calls slow
//         // the response time down far too much)
//         for ap_id in ap_ids {
//             if let Some((actor, _)) = get_actor(conn, ap_id.clone(), None, false).await {
//                 ap_actors.push(actor);
//             }
//         }

//         // set up the tuple structure for the ApNote conversion
//         let param: FullyQualifiedTimelineItem = (
//             (
//                 x.clone(),
//                 l.clone(),
//                 a.clone(),
//                 c.clone(),
//                 ra.clone(),
//                 rl.clone(),
//             ),
//             Some(ap_actors),
//         );

//         // convert to an ApNote
//         param.into()
//     }))
//     .await;

//     // use this to dedup the aforementioned duplicates
//     let mut consolidated_notes: HashMap<String, ApNote> = HashMap::new();

//     // this loop combines RemoteAnnounces and Likes in to the vecs that exists on
//     // the ApNote struct; the data from above will have one or no vec members per
//     // entry
//     for note in notes {
//         if let Some(id) = note.clone().id {
//             if consolidated_notes.contains_key(&id) {
//                 if let Some(consolidated) = consolidated_notes.get(&id) {
//                     let mut consolidated = consolidated.clone();

//                     if let (Some(mut existing), Some(announces)) = (
//                         consolidated.ephemeral_announces.clone(),
//                         note.ephemeral_announces,
//                     ) {
//                         existing.extend(announces);
//                         consolidated.ephemeral_announces = Some(existing);
//                     }

//                     if let (Some(mut existing), Some(likes)) =
//                         (consolidated.ephemeral_likes.clone(), note.ephemeral_likes)
//                     {
//                         existing.extend(likes);
//                         consolidated.ephemeral_likes = Some(existing);
//                     }

//                     consolidated_notes.insert(id, consolidated);
//                 }
//             } else {
//                 consolidated_notes.insert(id, note.clone());
//             }
//         }
//     }

//     // pull out the ApNote(s) from the consolidated map and reformat them as ApObject(s)
//     // to populate the ApCollection to send to the client
//     ApObject::Collection(ApCollection::from(
//         consolidated_notes
//             .values()
//             .map(|note| ApObject::Note(note.clone()))
//             .collect::<Vec<ApObject>>(),
//     ))
// }

async fn process(conn: &Db, timeline: Vec<AuthenticatedTimelineItem>) -> ApObject {
    let notes = process_notes(conn, &timeline).await;
    let consolidated_notes = consolidate_notes(notes);
    ApObject::Collection(ApCollection::from(
        consolidated_notes
            .iter()
            .map(|note| ApObject::Note(note.clone()))
            .collect::<Vec<ApObject>>(),
    ))
}

async fn process_notes(conn: &Db, timeline_items: &[AuthenticatedTimelineItem]) -> Vec<ApNote> {
    join_all(timeline_items.iter().map(
        |(timeline_item, like, announce, cc, remote_announce, remote_like)| async move {
            let ap_ids = gather_ap_ids(timeline_item);
            let ap_actors = get_ap_actors(conn, ap_ids).await;
            let fully_qualified_timeline_item: FullyQualifiedTimelineItem = (
                (
                    timeline_item.clone(),
                    like.clone(),
                    announce.clone(),
                    cc.clone(),
                    remote_announce.clone(),
                    remote_like.clone(),
                ),
                Some(ap_actors),
            );
            fully_qualified_timeline_item.into()
        },
    ))
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
        if let Some((actor, _)) = get_actor(conn, ap_id.clone(), None, false).await {
            ap_actors.push(actor);
        }
    }
    ap_actors
}

fn consolidate_notes(notes: Vec<ApNote>) -> Vec<ApNote> {
    let mut consolidated_notes: HashMap<String, ApNote> = HashMap::new();
    for note in notes {
        if let Some(id) = note.id.clone() {
            match consolidated_notes.entry(id) {
                Entry::Occupied(mut entry) => {
                    let consolidated = entry.get_mut();

                    if let (Some(existing), Some(announces)) = (
                        consolidated.ephemeral_announces.as_mut(),
                        note.ephemeral_announces,
                    ) {
                        existing.extend(announces);
                    }

                    if let (Some(existing), Some(likes)) =
                        (consolidated.ephemeral_likes.as_mut(), note.ephemeral_likes)
                    {
                        existing.extend(likes);
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
