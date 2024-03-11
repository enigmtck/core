use crate::activity_pub::{ApInstruments, ApNote, ApObject, ApSession};
use crate::db::Db;
use crate::schema::processing_queue;
use crate::POOL;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::encrypted_sessions::get_encrypted_session_by_profile_id_and_ap_to;
use super::profiles::Profile;
use super::remote_encrypted_sessions::RemoteEncryptedSession;
use super::remote_notes::RemoteNote;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = processing_queue)]
pub struct NewProcessingItem {
    pub profile_id: i32,
    pub kind: String,
    pub ap_id: String,
    pub ap_to: String,
    pub cc: Option<String>,
    pub attributed_to: String,
    pub ap_object: String,
    pub processed: bool,
}

type IdentifiedRemoteNote = (RemoteNote, i32);
impl From<IdentifiedRemoteNote> for NewProcessingItem {
    fn from(note: IdentifiedRemoteNote) -> Self {
        let (note, profile_id) = (note.0, note.1);
        let ap_note: ApNote = note.clone().into();

        NewProcessingItem {
            profile_id,
            kind: note.clone().kind.to_string(),
            ap_id: format!("{}#processing", note.ap_id),
            ap_to: note.clone().ap_to.unwrap(),
            attributed_to: note.clone().attributed_to,
            cc: note.cc,
            ap_object: serde_json::to_value(ap_note).unwrap(),
            processed: false,
        }
    }
}

impl From<RemoteEncryptedSession> for NewProcessingItem {
    fn from(session: RemoteEncryptedSession) -> Self {
        let ap_session: ApSession = session.clone().into();

        NewProcessingItem {
            profile_id: session.profile_id,
            kind: session.clone().kind,
            ap_id: format!("{}#processing", session.ap_id),
            ap_to: serde_json::to_value(&session.ap_to).unwrap(),
            attributed_to: session.attributed_to,
            cc: Option::None,
            ap_object: serde_json::to_value(ap_session).unwrap(),
            processed: false,
        }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = processing_queue)]
pub struct ProcessingItem {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub ap_id: String,
    pub ap_to: String,
    pub cc: Option<String>,
    pub attributed_to: String,
    pub kind: String,
    pub ap_object: String,
    pub processed: bool,
    pub profile_id: i32,
}

pub async fn create_processing_item(
    conn: Option<&Db>,
    processing_item: NewProcessingItem,
) -> Option<ProcessingItem> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                diesel::insert_into(processing_queue::table)
                    .values(&processing_item)
                    .get_result::<ProcessingItem>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            diesel::insert_into(processing_queue::table)
                .values(&processing_item)
                .get_result::<ProcessingItem>(&mut pool)
                .ok()
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

pub async fn resolve_processed_item_by_ap_id_and_profile_id(
    conn: &Db,
    profile_id: i32,
    ap_id: String,
) -> Option<ProcessingItem> {
    conn.run(move |c| {
        diesel::update(
            processing_queue::table
                .filter(processing_queue::ap_id.eq(ap_id))
                .filter(processing_queue::profile_id.eq(profile_id)),
        )
        .set(processing_queue::processed.eq(true))
        .get_result::<ProcessingItem>(c)
    })
    .await
    .ok()
}

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
