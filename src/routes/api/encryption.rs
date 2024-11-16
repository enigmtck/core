use std::collections::HashMap;

use crate::{
    activity_pub::{
        ActivityPub, ApCollection, ApCollectionPage, ApInstrument, ApObject, ApSession,
    },
    db::Db,
    fairings::signatures::Signed,
    models::{
        actors::get_actor_by_username,
        encrypted_sessions::EncryptedSession,
        olm_one_time_keys::{
            create_olm_one_time_key, get_next_otk_by_profile_id, get_otk_count_by_profile_id,
        },
        olm_sessions::OlmSession,
        pg::actors::update_olm_account_by_username,
        //processing_queue::{self, resolve_processed_item_by_ap_id_and_profile_id},
    },
    routes::ActivityJson,
    MaybeMultiple,
};
use anyhow::anyhow;
use base64::{engine::general_purpose, engine::Engine as _};
use rocket::{get, http::Status, post, serde::json::Error, serde::json::Json};
use serde::{Deserialize, Serialize};

// #[get("/api/user/<username>/sessions", format = "json")]
// pub async fn get_sessions(
//     //    signed: Signed,
//     conn: Db,
//     username: String,
// ) -> Result<Json<ApObject>, Status> {
//     //  if let Signed(true, VerificationType::Local) = signed {
//     let profile = get_actor_by_username(&conn, username)
//         .await
//         .ok_or(Status::NoContent)?;

//     let sessions: Vec<(EncryptedSession, Option<OlmSession>)> =
//         get_encrypted_sessions_by_profile_id(&conn, profile.id).await;

//     // this converts EncryptedSession to ApSession and (ApSession, Option<OlmSession>)
//     // into merged Vec<ApObject::Session> in one shot - see types/session.rs for details
//     let normalized: Vec<ActivityPub> = sessions
//         .iter()
//         .map(|(x, y)| ApObject::Session(((*x).clone().into(), (*y).clone()).into()))
//         .map(ActivityPub::from)
//         .collect();

//     Ok(Json(ApObject::CollectionPage(ApCollectionPage::from((
//         normalized, None,
//     )))))

//     // } else {
//     //     Err(Status::NoContent)
//     // }
// }

// #[get("/api/user/<username>/session/<encoded>")]
// pub async fn get_olm_session(
//     //    signed: Signed,
//     conn: Db,
//     username: String,
//     encoded: String,
// ) -> Result<Json<ApSession>, Status> {
//     //  if let Signed(true, VerificationType::Local) = signed {
//     let profile = get_actor_by_username(&conn, username)
//         .await
//         .ok_or(Status::NoContent)?;

//     let id = general_purpose::STANDARD.decode(encoded).map_err(|e| {
//         log::error!("FAILED TO DECODE id: {e:#?}");
//         Status::NoContent
//     })?;

//     let id = String::from_utf8(id).map_err(|e| {
//         log::error!("FAILED TO DECODE id: {e:#?}");
//         Status::NoContent
//     })?;

//     let (encrypted_session, olm_session) =
//         get_encrypted_session_by_profile_id_and_ap_to((&conn).into(), profile.id, id)
//             .await
//             .ok_or(Status::NoContent)?;

//     Ok(Json((encrypted_session.into(), olm_session).into()))

//     // } else {
//     //     Err(Status::NoContent)
//     // }
// }

// #[get("/api/user/<username>/queue")]
// pub async fn get_processing_queue(
//     signed: Signed,
//     conn: Db,
//     username: String,
// ) -> Result<Json<ApObject>, Status> {
//     if signed.local() {
//         let profile = get_actor_by_username(&conn, username)
//             .await
//             .ok_or(Status::NoContent)?;

//         let l = processing_queue::retrieve(&conn, profile)
//             .await
//             .iter()
//             .map(ActivityPub::from)
//             .collect();

//         Ok(Json(ApObject::CollectionPage(ApCollectionPage::from((
//             l, None,
//         )))))
//     } else {
//         Err(Status::NoContent)
//     }
// }

// #[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
// pub enum QueueTask {
//     Resolve,
//     #[default]
//     Unknown,
// }

// #[derive(Serialize, Deserialize, Default, Clone, Debug)]
// pub struct QueueAction {
//     id: String,
//     action: QueueTask,
// }

// #[post("/api/user/<username>/queue", format = "json", data = "<item>")]
// pub async fn update_processing_queue_item(
//     signed: Signed,
//     conn: Db,
//     username: String,
//     item: Result<Json<QueueAction>, Error<'_>>,
// ) -> Result<Status, Status> {
//     if signed.local() {
//         let Json(item) = item.map_err(|e| {
//             log::error!("FAILED TO DECODE item: {e:#?}");
//             Status::NoContent
//         })?;

//         let profile = get_actor_by_username(&conn, username)
//             .await
//             .ok_or(Status::NoContent)?;

//         if item.action == QueueTask::Resolve {
//             if resolve_processed_item_by_ap_id_and_profile_id(&conn, profile.id, item.id)
//                 .await
//                 .is_some()
//             {
//                 Ok(Status::Accepted)
//             } else {
//                 Err(Status::NoContent)
//             }
//         } else {
//             Err(Status::NoContent)
//         }
//     } else {
//         Err(Status::NoContent)
//     }
// }

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct OtkUpdateParams {
    pub keys: HashMap<String, String>,
    pub account: String,
    pub mutation_of: String,
    pub account_hash: String,
}

#[post("/api/user/<username>/otk", format = "json", data = "<params>")]
pub async fn add_one_time_keys(
    signed: Signed,
    conn: Db,
    username: String,
    params: Result<Json<OtkUpdateParams>, Error<'_>>,
) -> Result<Status, Status> {
    log::debug!("ADDING ONE-TIME-KEYS\n{params:#?}");

    let profile = signed.profile().ok_or(Status::Unauthorized)?;

    let Json(params) = params.map_err(|e| {
        log::error!("FAILED TO DECODE params: {e:#?}");
        Status::NoContent
    })?;

    if profile
        .ek_olm_pickled_account_hash
        .ok_or(Status::InternalServerError)?
        == params.mutation_of
    {
        if update_olm_account_by_username(&conn, username, params.account, params.account_hash)
            .await
            .is_some()
        {
            for (key, otk) in params.keys {
                create_olm_one_time_key(&conn, (profile.id, key, otk).into()).await;
            }

            Ok(Status::Accepted)
        } else {
            Err(Status::NoContent)
        }
    } else {
        log::error!("UNEXPECTED MUTATION");
        Err(Status::NoContent)
    }
}

#[get("/user/<username>/keys?<otk>", format = "application/activity+json")]
pub async fn keys_get(
    signed: Signed,
    conn: Db,
    username: String,
    otk: Option<bool>,
) -> Result<ActivityJson<ApObject>, Status> {
    let profile = get_actor_by_username(&conn, username.clone())
        .await
        .ok_or(Status::NotFound)?;

    // Requests for OTKs should come via the external path (i.e., signed.actor() not
    // signed.profile()). The WASM login process does call this endpoint to check for the
    // key population, so the 'else' path allows for unsigned access to the collection
    // description.
    if signed.actor().is_some_and(|x| x.id.is_some()) && otk.is_some_and(|x| x) {
        let otk = get_next_otk_by_profile_id(
            &conn,
            signed.actor().unwrap().id.unwrap().to_string(),
            profile.id,
        )
        .await
        .map_err(|e| {
            log::error!("Failed to get OTK: {e:#?}");
            Status::NotFound
        })?;

        let idk = ApInstrument::try_from(profile.clone()).map_err(|e| {
            log::error!("Failed to get IDK: {e:#?}");
            Status::InternalServerError
        })?;

        Ok(ActivityJson(Json(
            ApCollection::from(vec![otk.into(), idk]).into(),
        )))
    } else {
        let count = get_otk_count_by_profile_id(&conn, profile.id)
            .await
            .map_err(|e| {
                log::error!("Failed to retrieve OTK count: {e:#?}");
                Status::InternalServerError
            })?;

        let mut collection = ApCollection::default();
        collection.total_items = Some(count as i64);
        collection.id = Some(format!(
            "https://{}/user/{username}/keys",
            *crate::SERVER_NAME
        ));

        Ok(ActivityJson(Json(collection.into())))
    }
}
