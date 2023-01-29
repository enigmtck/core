use crate::{
    activity_pub::{ApCollection, ApNote, ApObject},
    db::Db,
    helper::get_ap_id_from_username,
    models::{profiles::Profile, timeline::get_timeline_items_by_ap_id_paged},
};

pub async fn timeline(conn: &Db, profile: Profile, limit: i64, offset: i64) -> ApObject {
    let timeline = get_timeline_items_by_ap_id_paged(
        conn,
        get_ap_id_from_username(profile.username),
        limit,
        offset,
    )
    .await;

    ApObject::Collection(ApCollection::from(
        timeline
            .iter()
            .map(|x| ApObject::Note(ApNote::from(x.clone())))
            .collect::<Vec<ApObject>>(),
    ))
}

// pub async fn all(conn: Db, profile: Profile) -> ApObject {
//     let mut consolidated: Vec<ApObject> = vec![];

//     consolidated.extend(timeline(&conn, profile.clone()).await);

//     ApObject::Collection(ApCollection::from(consolidated))
// }
