use crate::activity_pub::{ApInstruments, ApNoteType, ApObject, ApSession};
use crate::db::Db;
use crate::schema::processing_queue;
use diesel::prelude::*;

use super::actors::Actor;
use super::remote_encrypted_sessions::RemoteEncryptedSession;
use crate::models::encrypted_sessions::get_encrypted_session_by_profile_id_and_ap_to;
use crate::models::{from_serde, to_serde};

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        use crate::models::pg::notes::NoteType;
        pub fn to_kind(kind: ApNoteType) -> NoteType {
            kind.into()
        }

        pub use crate::models::pg::processing_queue::NewProcessingItem;
        pub use crate::models::pg::processing_queue::ProcessingItem;
        pub use crate::models::pg::processing_queue::create_processing_item;
        pub use crate::models::pg::processing_queue::resolve_processed_item_by_ap_id_and_profile_id;
    } else if #[cfg(feature = "sqlite")] {
        pub fn to_kind(kind: ApNoteType) -> String {
            kind.to_string().to_lowercase()
        }

        pub use crate::models::sqlite::processing_queue::NewProcessingItem;
        pub use crate::models::sqlite::processing_queue::ProcessingItem;
        pub use crate::models::sqlite::processing_queue::create_processing_item;
        pub use crate::models::sqlite::processing_queue::resolve_processed_item_by_ap_id_and_profile_id;
    }
}

impl From<RemoteEncryptedSession> for NewProcessingItem {
    fn from(session: RemoteEncryptedSession) -> Self {
        let ap_session: ApSession = session.clone().into();

        NewProcessingItem {
            profile_id: session.profile_id,
            kind: session.clone().kind,
            ap_id: format!("{}#processing", session.ap_id),
            ap_to: to_serde(session.ap_to).unwrap(),
            attributed_to: session.attributed_to,
            cc: Option::None,
            ap_object: to_serde(ap_session).unwrap(),
            processed: false,
        }
    }
}

pub async fn get_unprocessed_items_by_profile_id(conn: &Db, id: i32) -> Vec<ProcessingItem> {
    conn.run(move |c| {
        let query = processing_queue::table
            .filter(processing_queue::profile_id.eq(id))
            .filter(processing_queue::processed.eq(false))
            .order(processing_queue::created_at.asc())
            .into_boxed();

        query.get_results::<ProcessingItem>(c)
    })
    .await
    .unwrap_or(vec![])
}

pub async fn retrieve(conn: &Db, profile: Actor) -> Vec<ApObject> {
    let queue = get_unprocessed_items_by_profile_id(conn, profile.id).await;

    let objects: Vec<ApObject> = queue
        .iter()
        .filter_map(|v| from_serde::<ApObject>(v.clone().ap_object))
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
