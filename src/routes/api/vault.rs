use serde::{Deserialize, Serialize};

use crate::models::vault::VaultItem;

#[derive(Deserialize, Debug, Clone)]
pub struct SessionUpdate {
    pub session_uuid: String,
    pub encrypted_session: String,
    pub session_hash: String,
    pub mutation_of: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VaultStorageRequest {
    pub data: String,
    pub remote_actor: String,
    pub session: SessionUpdate,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VaultStorageResponse {
    pub uuid: Option<String>,
}

impl From<Option<VaultItem>> for VaultStorageResponse {
    fn from(item: Option<VaultItem>) -> Self {
        VaultStorageResponse {
            uuid: item.map(|x| x.uuid),
        }
    }
}

// #[post("/api/user/<username>/vault", data = "<data>")]
// pub async fn store_vault_item(
//     signed: Signed,
//     conn: Db,
//     username: String,
//     data: Json<VaultStorageRequest>,
// ) -> Result<Json<VaultStorageResponse>, Status> {
//     if !signed.local() {
//         return Err(Status::Unauthorized);
//     }

//     log::debug!("STORE VAULT REQUEST\n{data:#?}");

//     let profile = get_actor_by_username(&conn, username)
//         .await
//         .ok_or(Status::Unauthorized)?;

//     let data = data.0;
//     let session_update = data.clone().session;

//     if let Some((olm_session, Some(encrypted_session))) =
//         get_olm_session_by_uuid(&conn, session_update.session_uuid).await
//     {
//         if encrypted_session.profile_id == profile.id {
//             if update_olm_session_by_encrypted_session_id(
//                 &conn,
//                 olm_session.encrypted_session_id,
//                 session_update.encrypted_session,
//                 session_update.session_hash,
//             )
//             .await
//             .is_some()
//             {
//                 Ok(Json(
//                     create_vault_item(
//                         &conn,
//                         (data.clone().data, profile.id, data.clone().remote_actor).into(),
//                     )
//                     .await
//                     .into(),
//                 ))
//             } else {
//                 Err(Status::Unauthorized)
//             }
//         } else {
//             Err(Status::Unauthorized)
//         }
//     } else {
//         Err(Status::Unauthorized)
//     }
// }

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VaultRetrievalItem {
    pub created_at: String,
    pub updated_at: String,
    pub uuid: String,
    pub remote_actor: String,
    pub data: String,
}

// impl From<VaultItem> for VaultRetrievalItem {
//     fn from(item: VaultItem) -> Self {
//         cfg_if::cfg_if! {
//             if #[cfg(feature = "pg")] {
//                 let created_at = item.created_at.to_rfc3339();
//                 let updated_at = item.updated_at.to_rfc3339();
//             } else if #[cfg(feature = "sqlite")] {
//                 use chrono::{DateTime, Utc};

//                 let created_at = DateTime::<Utc>::from_naive_utc_and_offset(
//                     item.created_at,
//                     Utc,
//                 ).to_rfc3339();
//                 let updated_at = DateTime::<Utc>::from_naive_utc_and_offset(
//                     item.updated_at,
//                     Utc,
//                 ).to_rfc3339();
//             }
//         }

//         VaultRetrievalItem {
//             created_at,
//             updated_at,
//             uuid: item.uuid,
//             remote_actor: item.remote_actor,
//             data: item.encrypted_data,
//         }
//     }
// }

// #[get("/api/user/<username>/vault?<offset>&<limit>&<actor>")]
// pub async fn vault_get(
//     signed: Signed,
//     conn: Db,
//     username: String,
//     offset: u16,
//     limit: u8,
//     actor: String,
// ) -> Result<Json<ApObject>, Status> {
//     if !signed.local() {
//         return Err(Status::Unauthorized);
//     }

//     let profile = get_actor_by_username(&conn, username)
//         .await
//         .ok_or(Status::Unauthorized)?;

//     let actor = general_purpose::STANDARD.decode(actor).map_err(|e| {
//         log::error!("FAILED TO DECODE Actor: {e:#?}");
//         Status::UnprocessableEntity
//     })?;

//     let items: Vec<VaultItem> = get_vault_items_by_profile_id_and_remote_actor(
//         &conn,
//         profile.id,
//         limit.into(),
//         offset.into(),
//         String::from_utf8(actor).unwrap(),
//     )
//     .await;

//     Ok(Json(ApObject::Collection(ApCollection::from(
//         (items, profile) as IdentifiedVaultItems,
//     ))))
// }
