use rocket::futures::future::join_all;

use crate::{
    activity_pub::{retriever::get_actor, ApActor, ApCollection, ApNote, ApObject, ApTag},
    db::Db,
    fairings::faktory::{assign_to_faktory, FaktoryConnection},
    models::timeline::{get_public_timeline_items, get_timeline_items_by_conversation},
};

pub async fn timeline(conn: &Db, limit: i64, offset: i64) -> ApObject {
    let timeline = get_public_timeline_items(conn, limit, offset).await;

    ApObject::Collection(ApCollection::from(
        join_all(timeline.iter().map(|x| async {
            let mut ap_ids = vec![x.clone().attributed_to];
            let mut ap_actors: Vec<ApActor> = vec![];

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

            for ap_id in ap_ids {
                if let Some((actor, _)) = get_actor(conn, ap_id, None).await {
                    ap_actors.push(actor.into());
                }
            }

            ApObject::Note(ApNote::from((x.clone(), Some(ap_actors))))
        }))
        .await,
    ))
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
