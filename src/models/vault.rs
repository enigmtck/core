use crate::activity_pub::ApInstrument;
use crate::db::Db;
use crate::schema::vault;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::Insertable;
use diesel::{AsChangeset, Identifiable, Queryable};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

use super::actors::Actor;

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = vault)]
pub struct VaultItem {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub owner_as_id: String,
    pub activity_id: i32,
    pub data: String,
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = vault)]
pub struct NewVaultItem {
    pub uuid: String,
    pub owner_as_id: String,
    pub activity_id: i32,
    pub data: String,
}

pub struct VaultItemParams {
    pub instrument: ApInstrument,
    pub owner: Actor,
    pub activity_id: i32,
}

impl TryFrom<VaultItemParams> for NewVaultItem {
    type Error = anyhow::Error;

    fn try_from(
        VaultItemParams {
            instrument,
            owner,
            activity_id,
        }: VaultItemParams,
    ) -> Result<Self, Self::Error> {
        if !instrument.is_vault_item() {
            return Err(anyhow!("Instrument is not a VaultItem"));
        }

        Ok(NewVaultItem {
            uuid: uuid::Uuid::new_v4().to_string(),
            owner_as_id: owner.as_id,
            data: instrument
                .content
                .ok_or_else(|| anyhow!("Instrument must have content"))?,
            activity_id,
        })
    }
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

pub async fn create_vault_item(conn: &Db, vault_item: NewVaultItem) -> Result<VaultItem> {
    conn.run(move |c| {
        diesel::insert_into(vault::table)
            .values(&vault_item)
            .get_result::<VaultItem>(c)
    })
    .await
    .map_err(anyhow::Error::msg)
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
