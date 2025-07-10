use crate::models::vault::VaultItem;

use serde::{Deserialize, Serialize};

// #[derive(Deserialize, Debug, Clone)]
// pub struct SessionUpdate {
//     pub session_uuid: String,
//     pub encrypted_session: String,
//     pub session_hash: String,
//     pub mutation_of: String,
// }

// #[derive(Deserialize, Debug, Clone)]
// pub struct VaultStorageRequest {
//     pub data: String,
//     pub remote_actor: String,
//     pub session: SessionUpdate,
// }

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VaultStorageResponse {
    pub uuid: Option<String>,
}

impl From<Result<VaultItem, anyhow::Error>> for VaultStorageResponse {
    fn from(item: Result<VaultItem, anyhow::Error>) -> Self {
        VaultStorageResponse {
            uuid: item.ok().map(|x| x.uuid),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VaultRetrievalItem {
    pub created_at: String,
    pub updated_at: String,
    pub uuid: String,
    pub remote_actor: String,
    pub data: String,
}
