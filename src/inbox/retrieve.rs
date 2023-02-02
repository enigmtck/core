use crate::{
    activity_pub::{ApCollection, ApNote, ApObject},
    db::Db,
    models::timeline::{get_public_timeline_items, get_timeline_items_by_conversation},
};

pub async fn timeline(conn: &Db, limit: i64, offset: i64) -> ApObject {
    let timeline = get_public_timeline_items(conn, limit, offset).await;

    ApObject::Collection(ApCollection::from(
        timeline
            .iter()
            .map(|x| ApObject::Note(ApNote::from(x.clone())))
            .collect::<Vec<ApObject>>(),
    ))
}

pub async fn conversation(conn: &Db, conversation: String, limit: i64, offset: i64) -> ApObject {
    let conversation = get_timeline_items_by_conversation(conn, conversation, limit, offset).await;

    ApObject::Collection(ApCollection::from(
        conversation
            .iter()
            .map(|x| ApObject::Note(ApNote::from(x.clone())))
            .collect::<Vec<ApObject>>(),
    ))
}
