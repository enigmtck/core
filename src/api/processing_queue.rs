use crate::{
    activity_pub::ApObject,
    db::Db,
    models::{processing_queue::get_unprocessed_items_by_profile_id, profiles::Profile},
};

pub async fn retrieve(conn: &Db, profile: Profile) -> Vec<ApObject> {
    let queue = get_unprocessed_items_by_profile_id(conn, profile.id).await;

    queue
        .iter()
        .map(|x| serde_json::from_value(x.clone().ap_object).unwrap())
        .collect()
}
