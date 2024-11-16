use crate::activity_pub::ApNote;
use crate::db::Db;
use crate::schema::vault;
use crate::MaybeMultiple;
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
    pub uuid: String,
    pub owner_as_id: String,
    pub activity_id: i32,
    pub data: String,
}

type EncryptedData = (String, String, i32);
impl From<EncryptedData> for NewVaultItem {
    fn from((data, owner_as_id, activity_id): EncryptedData) -> Self {
        NewVaultItem {
            data,
            owner_as_id,
            activity_id,
            uuid: uuid::Uuid::new_v4().to_string(),
        }
    }
}

pub async fn get_vault_items_by_owner_as_id(
    conn: &Db,
    owner_as_id: String,
    limit: i64,
    offset: i64,
) -> Vec<VaultItem> {
    conn.run(move |c| {
        let query = vault::table
            .filter(vault::owner_as_id.eq(owner_as_id))
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
