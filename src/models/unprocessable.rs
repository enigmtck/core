use crate::db::runner::DbRunner;
use crate::schema::unprocessable;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::{AsChangeset, Identifiable, Queryable};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = unprocessable)]
pub struct NewUnprocessable {
    pub raw: Value,
    pub error: Option<String>,
}

impl From<Value> for NewUnprocessable {
    fn from(value: Value) -> Self {
        NewUnprocessable {
            raw: value,
            error: None,
        }
    }
}

pub type UnprocessableFields = (Value, Option<String>);
impl From<UnprocessableFields> for NewUnprocessable {
    fn from((raw, error): UnprocessableFields) -> Self {
        NewUnprocessable { raw, error }
    }
}

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = unprocessable)]
pub struct Unprocessable {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub raw: Value,
    pub error: Option<String>,
}

pub async fn create_unprocessable<C: DbRunner>(
    conn: &C,
    unprocessable: NewUnprocessable,
) -> Option<Unprocessable> {
    conn.run(move |c| {
        diesel::insert_into(unprocessable::table)
            .values(&unprocessable)
            .get_result::<Unprocessable>(c)
    })
    .await
    .ok()
}
