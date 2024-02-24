use crate::{
    activity_pub::{ApInstruments, ApObject},
    db::Db,
    models::{
        encrypted_sessions::get_encrypted_session_by_profile_id_and_ap_to,
        processing_queue::get_unprocessed_items_by_profile_id, profiles::Profile,
    },
};

pub async fn retrieve(conn: &Db, profile: Profile) -> Vec<ApObject> {
    let queue = get_unprocessed_items_by_profile_id(conn, profile.id).await;

    let objects: Vec<ApObject> = queue
        .iter()
        .filter_map(|v| serde_json::from_value::<ApObject>(v.clone().ap_object).ok())
        .collect();

    let mut returned: Vec<ApObject> = vec![];

    for object in objects.clone() {
        if let ApObject::Note(mut note) = object.clone() {
            log::debug!(
                "LOOKING FOR profile {} AND ap_to {}",
                profile.id,
                note.clone().attributed_to
            );
            if let Some(session) = get_encrypted_session_by_profile_id_and_ap_to(
                conn.into(),
                profile.id,
                note.clone().attributed_to.to_string(),
            )
            .await
            {
                log::debug!("FOUND ENCRYPTED SESSION\n{session:#?}");
                if let Some(olm_session) = session.1 {
                    log::debug!("FOUND OLM SESSION\n{olm_session:#?}");
                    note.instrument = Some(ApInstruments::Single(olm_session.into()));
                }
            }

            log::debug!("PUSHING NOTE WITH SESSION\n{note:#?}");
            returned.push(ApObject::Note(note));
        } else {
            returned.push(object.clone());
        }
    }

    returned
}
