use crate::{
    activity_pub::{ApCollection, ApNote, ApObject},
    db::Db,
    helper::get_ap_id_from_username,
    models::{profiles::Profile, timeline::get_timeline_items_by_ap_id},
};

pub async fn timeline(conn: &Db, profile: Profile) -> Vec<ApObject> {
    let timeline =
        get_timeline_items_by_ap_id(conn, get_ap_id_from_username(profile.username)).await;

    timeline
        .iter()
        .map(|(x, _, _)| ApObject::Note(ApNote::from(x.clone())))
        .collect()
}

pub async fn all(conn: Db, profile: Profile) -> ApObject {
    let mut consolidated: Vec<ApObject> = vec![];

    consolidated.extend(timeline(&conn, profile.clone()).await);

    ApObject::Collection(ApCollection::from(consolidated))
}
