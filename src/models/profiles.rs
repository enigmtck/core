use crate::db::{jsonb_set, Db};
use crate::schema::{self, profiles};
use chrono::{DateTime, Utc};
use diesel::deserialize::FromSql;
use diesel::pg::types::sql_types::Jsonb;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::serialize::{Output, ToSql};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use rocket_sync_db_pools::{database, diesel};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Write;

#[derive(Serialize, Deserialize, Insertable, Default)]
#[table_name = "profiles"]
pub struct NewProfile {
    pub uuid: String,
    pub username: String,
    pub display_name: String,
    pub summary: Option<String>,
    pub public_key: String,
    pub private_key: String,
    pub password: Option<String>,
    pub keystore: Option<Value>,
    pub client_public_key: Option<String>,
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Debug, Default)]
#[table_name = "profiles"]
pub struct Profile {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub username: String,
    pub display_name: String,
    pub summary: Option<String>,
    pub public_key: String,
    #[serde(skip_serializing)]
    pub private_key: String,
    #[serde(skip_serializing)]
    pub password: Option<String>,
    pub keystore: KeyStore,
    pub client_public_key: Option<String>,
    pub avatar_filename: String,
    pub banner_filename: Option<String>,
}

#[derive(FromSqlRow, AsExpression, serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
#[sql_type = "Jsonb"]
pub struct KeyStore {
    pub salt: Option<String>,
    pub client_private_key: Option<String>,
    pub olm_identity_public_key: Option<String>,
    pub olm_one_time_keys: Option<HashMap<String, Vec<u8>>>,
    pub olm_pickled_account: Option<String>,
    pub olm_external_identity_keys: Option<HashMap<String, String>>,
    pub olm_external_one_time_keys: Option<HashMap<String, String>>,
    pub olm_sessions: Option<String>,
}

impl FromSql<Jsonb, Pg> for KeyStore {
    fn from_sql(bytes: Option<&[u8]>) -> diesel::deserialize::Result<Self> {
        let value = <serde_json::Value as FromSql<Jsonb, Pg>>::from_sql(bytes)?;
        Ok(serde_json::from_value(value)?)
    }
}

impl ToSql<Jsonb, Pg> for KeyStore {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> diesel::serialize::Result {
        let value = serde_json::to_value(self)?;
        <serde_json::Value as ToSql<Jsonb, Pg>>::to_sql(&value, out)
    }
}

pub async fn update_otk_by_username(
    conn: &Db,
    username: String,
    keystore: KeyStore,
) -> Option<Profile> {
    use schema::profiles::dsl::{keystore as k, profiles, username as u};

    match conn
        .run(move |c| {
            diesel::update(profiles.filter(u.eq(username)))
                .set(k.eq(jsonb_set(
                    k,
                    vec![String::from("olm_one_time_keys")],
                    serde_json::to_value(&keystore.olm_one_time_keys).unwrap(),
                )))
                .get_result::<Profile>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}

pub async fn update_olm_external_identity_keys_by_username(
    conn: &Db,
    username: String,
    keystore: KeyStore,
) -> Option<Profile> {
    use schema::profiles::dsl::{keystore as k, profiles, username as u};

    match conn
        .run(move |c| {
            diesel::update(profiles.filter(u.eq(username)))
                .set(k.eq(jsonb_set(
                    k,
                    vec![String::from("olm_external_identity_keys")],
                    serde_json::to_value(&keystore.olm_external_identity_keys).unwrap(),
                )))
                .get_result::<Profile>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}

pub async fn update_olm_external_one_time_keys_by_username(
    conn: &Db,
    username: String,
    keystore: KeyStore,
) -> Option<Profile> {
    use schema::profiles::dsl::{keystore as k, profiles, username as u};

    match conn
        .run(move |c| {
            diesel::update(profiles.filter(u.eq(username)))
                .set(k.eq(jsonb_set(
                    k,
                    vec![String::from("olm_external_one_time_keys")],
                    serde_json::to_value(&keystore.olm_external_one_time_keys).unwrap(),
                )))
                .get_result::<Profile>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}

pub async fn update_olm_sessions_by_username(
    conn: &Db,
    username: String,
    keystore: KeyStore,
) -> Option<Profile> {
    use schema::profiles::dsl::{keystore as k, profiles, username as u};

    match conn
        .run(move |c| {
            diesel::update(profiles.filter(u.eq(username)))
                .set(k.eq(jsonb_set(
                    k,
                    vec![String::from("olm_sessions")],
                    serde_json::to_value(&keystore.olm_sessions).unwrap(),
                )))
                .get_result::<Profile>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}
