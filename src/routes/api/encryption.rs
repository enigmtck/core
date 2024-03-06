use std::collections::HashMap;

use crate::{
    activity_pub::{ApCollection, ApObject, ApSession},
    db::Db,
    fairings::signatures::Signed,
    models::{
        encrypted_sessions::{
            get_encrypted_session_by_profile_id_and_ap_to, get_encrypted_sessions_by_profile_id,
            EncryptedSession,
        },
        olm_one_time_keys::create_olm_one_time_key,
        olm_sessions::OlmSession,
        processing_queue::{self, resolve_processed_item_by_ap_id_and_profile_id},
        profiles::{get_profile_by_username, update_olm_account_by_username},
    },
    signing::VerificationType,
};
use base64::{engine::general_purpose, engine::Engine as _};
use rocket::{get, http::Status, post, serde::json::Error, serde::json::Json};
use serde::{Deserialize, Serialize};

#[get("/api/user/<username>/sessions", format = "json")]
pub async fn get_sessions(
    //    signed: Signed,
    conn: Db,
    username: String,
) -> Result<Json<ApObject>, Status> {
    //  if let Signed(true, VerificationType::Local) = signed {
    if let Some(profile) = get_profile_by_username((&conn).into(), username).await {
        let sessions: Vec<(EncryptedSession, Option<OlmSession>)> =
            get_encrypted_sessions_by_profile_id(&conn, profile.id).await;

        // this converts EncryptedSession to ApSession and (ApSession, Option<OlmSession>)
        // into merged Vec<ApObject::Session> in one shot - see types/session.rs for details
        let normalized: Vec<ApObject> = sessions
            .iter()
            .map(|(x, y)| ApObject::Session(((*x).clone().into(), (*y).clone()).into()))
            .collect();

        Ok(Json(ApObject::Collection(ApCollection::from(normalized))))
    } else {
        Err(Status::NoContent)
    }
    // } else {
    //     Err(Status::NoContent)
    // }
}

#[get("/api/user/<username>/session/<encoded>")]
pub async fn get_olm_session(
    //    signed: Signed,
    conn: Db,
    username: String,
    encoded: String,
) -> Result<Json<ApSession>, Status> {
    //  if let Signed(true, VerificationType::Local) = signed {
    if let Some(profile) = get_profile_by_username((&conn).into(), username).await {
        if let Ok(id) = general_purpose::STANDARD.decode(encoded) {
            if let Ok(id) = String::from_utf8(id) {
                if let Some((encrypted_session, olm_session)) =
                    get_encrypted_session_by_profile_id_and_ap_to((&conn).into(), profile.id, id)
                        .await
                {
                    Ok(Json((encrypted_session.into(), olm_session).into()))
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
    // } else {
    //     Err(Status::NoContent)
    // }
}
#[get("/api/user/<username>/queue")]
pub async fn get_processing_queue(
    signed: Signed,
    conn: Db,
    username: String,
) -> Result<Json<ApObject>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Some(profile) = get_profile_by_username((&conn).into(), username).await {
            let l = processing_queue::retrieve(&conn, profile).await;

            Ok(Json(ApObject::Collection(ApCollection::from(l))))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub enum QueueTask {
    Resolve,
    #[default]
    Unknown,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct QueueAction {
    id: String,
    action: QueueTask,
}

#[post("/api/user/<username>/queue", format = "json", data = "<item>")]
pub async fn update_processing_queue_item(
    signed: Signed,
    conn: Db,
    username: String,
    item: Result<Json<QueueAction>, Error<'_>>,
) -> Result<Status, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Ok(Json(item)) = item {
            if let Some(profile) = get_profile_by_username((&conn).into(), username).await {
                if item.action == QueueTask::Resolve {
                    if resolve_processed_item_by_ap_id_and_profile_id(&conn, profile.id, item.id)
                        .await
                        .is_some()
                    {
                        Ok(Status::Accepted)
                    } else {
                        Err(Status::NoContent)
                    }
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

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

    if let Signed(true, VerificationType::Local) = signed {
        if let Some(profile) = get_profile_by_username((&conn).into(), username.clone()).await {
            if let Ok(Json(params)) = params {
                if profile.olm_pickled_account_hash == params.mutation_of.into() {
                    if update_olm_account_by_username(
                        &conn,
                        username,
                        params.account,
                        params.account_hash,
                    )
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
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}
