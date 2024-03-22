use crate::db::Db;
use crate::models::vault::NewVaultItem;
use crate::schema::vault;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::Serialize;

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = vault)]
pub struct VaultItem {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub uuid: String,
    pub profile_id: i32,
    pub encrypted_data: String,
    pub remote_actor: String,
    pub outbound: bool,
}

pub async fn create_vault_item(conn: &Db, vault_item: NewVaultItem) -> Option<VaultItem> {
    conn.run(move |c| {
        diesel::insert_into(vault::table)
            .values(&vault_item)
            .execute(c)
            .ok()?;

        vault::table
            .order(vault::id.desc())
            .first::<VaultItem>(c)
            .ok()
    })
    .await
}
