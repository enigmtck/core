use crate::db::Db;
use crate::schema::vault;
use diesel::prelude::*;
use diesel::Insertable;
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub use crate::models::pg::vault::VaultItem;
        pub use crate::models::pg::vault::create_vault_item;
    } else if #[cfg(feature = "sqlite")] {
        pub use crate::models::sqlite::vault::VaultItem;
        pub use crate::models::sqlite::vault::create_vault_item;
    }
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = vault)]
pub struct NewVaultItem {
    pub profile_id: i32,
    pub uuid: String,
    pub encrypted_data: String,
    pub remote_actor: String,
}

type EncryptedData = (String, i32, String);
impl From<EncryptedData> for NewVaultItem {
    fn from((encrypted_data, profile_id, remote_actor): EncryptedData) -> Self {
        NewVaultItem {
            profile_id,
            encrypted_data,
            remote_actor,
            uuid: uuid::Uuid::new_v4().to_string(),
        }
    }
}

pub async fn get_vault_items_by_profile_id_and_remote_actor(
    conn: &Db,
    id: i32,
    limit: i64,
    offset: i64,
    actor: String,
) -> Vec<VaultItem> {
    conn.run(move |c| {
        let query = vault::table
            .filter(vault::profile_id.eq(id))
            .filter(vault::remote_actor.eq(actor))
            .order(vault::created_at.desc())
            .limit(limit)
            .offset(offset)
            .into_boxed();

        query.get_results::<VaultItem>(c)
    })
    .await
    .unwrap_or(vec![])
}

pub async fn get_vault_items_by_profile_id(
    conn: &Db,
    id: i32,
    limit: i64,
    offset: i64,
) -> Vec<VaultItem> {
    conn.run(move |c| {
        let query = vault::table
            .filter(vault::profile_id.eq(id))
            .order(vault::created_at.desc())
            .limit(limit)
            .offset(offset)
            .into_boxed();

        query.get_results::<VaultItem>(c)
    })
    .await
    .unwrap_or(vec![])
}

pub async fn get_vault_item_by_uuid(conn: &Db, uuid: String) -> Option<VaultItem> {
    conn.run(move |c| {
        let query = vault::table.filter(vault::uuid.eq(uuid));

        query.first::<VaultItem>(c)
    })
    .await
    .ok()
}
