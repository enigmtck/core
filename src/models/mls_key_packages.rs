use crate::db::Db;
use crate::helper::get_instrument_as_id_from_uuid;
use crate::schema::mls_key_packages;
use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::Insertable;
use diesel::{AsChangeset, Identifiable, Queryable};
use jdt_activity_pub::{ApInstrument, ApInstrumentType};
use rocket_sync_db_pools::diesel;
use serde::{Deserialize, Serialize};

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = mls_key_packages)]
pub struct MlsKeyPackage {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub actor_id: i32,
    pub key_data: String,
    pub distributed: bool,
    pub assignee: Option<String>,
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = mls_key_packages)]
pub struct NewMlsKeyPackage {
    pub uuid: String,
    pub actor_id: i32,
    pub key_data: String,
    pub distributed: bool,
    pub assignee: Option<String>,
}

impl From<MlsKeyPackage> for ApInstrument {
    fn from(mkp: MlsKeyPackage) -> Self {
        Self {
            kind: ApInstrumentType::MlsKeyPackage,
            id: Some(get_instrument_as_id_from_uuid(mkp.uuid.clone())),
            content: Some(mkp.key_data),
            uuid: None,
            hash: None,
            name: None,
            url: None,
            mutation_of: None,
            conversation: None,
            activity: None,
        }
    }
}

// profile_id, key_data
type KeyTuple = (i32, String);

impl From<KeyTuple> for NewMlsKeyPackage {
    fn from((actor_id, key_data): KeyTuple) -> NewMlsKeyPackage {
        NewMlsKeyPackage {
            actor_id,
            uuid: uuid::Uuid::new_v4().to_string(),
            key_data,
            distributed: false,
            assignee: None,
        }
    }
}

pub async fn create_mls_key_package(
    conn: &Db,
    mls_key_package: NewMlsKeyPackage,
) -> Result<MlsKeyPackage> {
    conn.run(move |c| {
        diesel::insert_into(mls_key_packages::table)
            .values(&mls_key_package)
            .get_result::<MlsKeyPackage>(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn get_mls_key_packages_by_actor_id(
    conn: &Db,
    id: i32,
    limit: i64,
    offset: i64,
) -> Vec<MlsKeyPackage> {
    conn.run(move |c| {
        let query = mls_key_packages::table
            .filter(mls_key_packages::actor_id.eq(id))
            .order(mls_key_packages::created_at.asc())
            .limit(limit)
            .offset(offset)
            .into_boxed();

        query.get_results::<MlsKeyPackage>(c)
    })
    .await
    .unwrap_or(vec![])
}

pub async fn get_next_mkp_by_actor_id(
    conn: &Db,
    actor_as_id: String,
    id: i32,
) -> Result<MlsKeyPackage> {
    conn.run(move |c| {
        mls_key_packages::table
            .filter(
                mls_key_packages::actor_id
                    .eq(id)
                    .and(mls_key_packages::distributed.eq(false)),
            )
            .order(mls_key_packages::created_at.asc())
            .first::<MlsKeyPackage>(c)
            .and_then(|mkp| {
                diesel::update(mls_key_packages::table.find(mkp.id))
                    .set((
                        mls_key_packages::distributed.eq(true),
                        mls_key_packages::assignee.eq(actor_as_id),
                    ))
                    .get_result::<MlsKeyPackage>(c)
            })
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn get_mkp_count_by_profile_id(conn: &Db, id: i32) -> Result<i64> {
    conn.run(move |c| {
        mls_key_packages::table
            .filter(
                mls_key_packages::actor_id
                    .eq(id)
                    .and(mls_key_packages::distributed.eq(false)),
            )
            .count()
            .get_result(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}
