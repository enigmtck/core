use crate::models::notes::{NewNote, Note};
use crate::models::profiles::Profile;
use crate::models::remote_encrypted_sessions::{NewRemoteEncryptedSession, RemoteEncryptedSession};
use crate::schema;
use diesel::prelude::*;
use rocket_sync_db_pools::{database, diesel};

// this is a reference to the value in Rocket.toml, not the actual
// database name
#[database("enigmatick")]
pub struct Db(diesel::PgConnection);

pub async fn create_remote_encrypted_session(
    conn: &Db,
    remote_encrypted_session: NewRemoteEncryptedSession,
) -> Option<RemoteEncryptedSession> {
    use schema::remote_encrypted_sessions;

    if let Ok(x) = conn
        .run(move |c| {
            diesel::insert_into(remote_encrypted_sessions::table)
                .values(&remote_encrypted_session)
                .get_result::<RemoteEncryptedSession>(c)
        })
        .await
    {
        Some(x)
    } else {
        Option::None
    }
}

pub async fn get_remote_encrypted_session_by_ap_id(
    conn: &Db,
    apid: String,
) -> Option<RemoteEncryptedSession> {
    use self::schema::remote_encrypted_sessions::dsl::{ap_id, remote_encrypted_sessions};

    if let Ok(x) = conn
        .run(move |c| {
            remote_encrypted_sessions
                .filter(ap_id.eq(apid))
                .first::<RemoteEncryptedSession>(c)
        })
        .await
    {
        Option::from(x)
    } else {
        Option::None
    }
}

// pub async fn get_remote_notes_by_profile_id(conn: &Db, id: i32) -> Vec<RemoteNote> {
//     use self::schema::remote_notes::dsl::{profile_id, remote_notes};

//     match conn
//         .run(move |c| {
//             remote_notes
//                 .filter(profile_id.eq(id))
//                 .get_results::<RemoteNote>(c)
//         })
//         .await
//     {
//         Ok(x) => x,
//         Err(_) => vec![],
//     }
// }

pub async fn create_note(conn: &Db, note: NewNote) -> Option<Note> {
    use schema::notes;

    if let Ok(x) = conn
        .run(move |c| {
            diesel::insert_into(notes::table)
                .values(&note)
                .get_result::<Note>(c)
        })
        .await
    {
        Some(x)
    } else {
        Option::None
    }
}

// pub async fn create_remote_note(conn: &Db, remote_note: NewRemoteNote) -> Option<RemoteNote> {
//     use schema::remote_notes;

//     if let Ok(x) = conn
//         .run(move |c| {
//             diesel::insert_into(remote_notes::table)
//                 .values(&remote_note)
//                 .on_conflict(remote_notes::ap_id)
//                 .do_nothing()
//                 .get_result::<RemoteNote>(c)
//         })
//         .await
//     {
//         Some(x)
//     } else {
//         Option::None
//     }
// }
